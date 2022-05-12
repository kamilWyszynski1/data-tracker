use crate::core::task::InputData;
use crate::error::types::{Error, Result};
use serde_json::Value;
use std::{collections::HashMap, fmt};

#[derive(Debug, PartialEq, Clone)]
pub enum Variable {
    None, // placeholder for functionalities that does not produce Variables, like DEFINE.
    Bool(bool),
    Int(isize),
    Float(f32),
    String(String),
    Vector(Vec<Variable>),
    Object(HashMap<String, Variable>),
    Json(Value),
}

impl Variable {
    /// Translated InputData enum to Variable enum.
    pub fn from_input_data(td: &InputData) -> Self {
        match td {
            InputData::String(s) => Variable::String(s.clone()),
            InputData::Json(j) => Variable::Json(j.clone()),
            InputData::Vector(v) => Variable::Vector(v.iter().map(Self::from_input_data).collect()),
        }
    }

    pub fn to_str(&self) -> Result<&str> {
        match self {
            Variable::String(string) => Ok(&string),
            _ => Err(Error::new_internal(
                String::from("Variable::to_str"),
                String::from(""),
                String::from("Variable cannot be converted to str"),
            )),
        }
    }

    /// Returns true if Variable is: Bool(true), Int(1), Float(1.0);
    pub fn is_true(&self) -> bool {
        match self {
            Variable::Bool(bool) => *bool,
            Variable::Int(int) => *int == 1,
            Variable::Float(float) => *float == 1.0,
            _ => false,
        }
    }
}

impl fmt::Display for Variable {
    // This trait requires `fmt` with this exact signature.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let buf = match self {
            Variable::None => String::from("None"),
            Variable::Bool(_) => String::from("Bool"),
            Variable::Int(_) => String::from("Int"),
            Variable::Float(_) => String::from("Float"),
            Variable::String(_) => String::from("String"),
            Variable::Vector(_) => String::from("Vector"),
            Variable::Object(_) => String::from("Object"),
            Variable::Json(_) => String::from("Json"),
        };
        // Write strictly the first element into the supplied output
        // stream: `f`. Returns `fmt::Result` which indicates whether the
        // operation succeeded or failed. Note that `write!` uses syntax which
        // is very similar to `println!`.
        write!(f, "{}", buf)
    }
}

fn serde_value_to_variable(v: Value) -> Variable {
    if v.is_boolean() {
        return Variable::Bool(v.as_bool().unwrap());
    } else if v.is_string() {
        return Variable::String(v.as_str().unwrap().to_string());
    } else if v.is_i64() {
        return Variable::Int(v.as_i64().unwrap() as isize);
    } else if v.is_f64() {
        return Variable::Float(v.as_f64().unwrap() as f32);
    } else if v.is_array() {
        let vec: Vec<Variable> = v
            .as_array()
            .unwrap()
            .iter()
            .map(|v| serde_value_to_variable(v.clone()))
            .collect();
        return Variable::Vector(vec);
    } else if v.is_object() {
        return value_object_to_variable_object(v);
    }
    Variable::None
}

pub fn value_object_to_variable_object(v: Value) -> Variable {
    let m: HashMap<String, Variable> = v
        .as_object()
        .unwrap()
        .into_iter()
        .map(|(k, v)| (k.clone(), serde_value_to_variable(v.clone())))
        .collect();
    Variable::Object(m)
}

impl Variable {
    /// Extract one variable from another, nested one.
    /// deep parameter indicates if we want to 'loop' extraction.
    pub fn extract(&self, f: &Variable, deep: bool) -> Result<Variable> {
        let extracted = match *self {
            Variable::None
            | Variable::Bool(_)
            | Variable::Int(_)
            | Variable::Float(_)
            | Variable::String(_) => {
                if deep {
                    // we get to non extractable variable, return.
                    return Ok(self.clone());
                }
                return Err(Error::new_internal(
                    String::from("Variable::extract"),
                    String::from(""),
                    String::from("non extractable variable"),
                ));
            }
            Variable::Vector(ref vec) => {
                if vec.len() == 0 {
                    return Ok(Variable::None);
                }
                let inx = match f {
                    Variable::Int(i) => *i as usize,
                    Variable::String(s) => s.parse().unwrap(),
                    _ => {
                        return Err(Error::new_internal(
                            String::from("Variable::extract"),
                            String::from(""),
                            String::from("invalid index type"),
                        ))
                    }
                };

                let inx = inx as usize;
                if inx > 0 && vec.len() < inx {
                    return Err(Error::new_internal(
                        String::from("Variable::extract"),
                        String::from(""),
                        String::from("index out of range"),
                    ));
                }
                vec[inx].clone()
            }
            Variable::Object(ref obj) => {
                if let Variable::String(key) = f {
                    match obj.get(key) {
                        Some(v) => v.clone(),
                        None => {
                            return Err(Error::new_internal(
                                String::from("Variable::extract"),
                                String::from(""),
                                format!("object does not have {} field", key),
                            ))
                        }
                    }
                } else {
                    return Err(Error::new_internal(
                        String::from("Variable::extract"),
                        String::from(""),
                        String::from("f is not Variable::String"),
                    ));
                }
            }
            Variable::Json(ref jsn) => {
                if let Variable::String(key) = f {
                    match jsn.get(key) {
                        Some(v) => serde_value_to_variable(v.clone()),
                        None => {
                            return Err(Error::new_internal(
                                String::from("Variable::extract"),
                                String::from(""),
                                format!("json does not have {} field", key),
                            ))
                        }
                    }
                } else {
                    return Err(Error::new_internal(
                        String::from("Variable::extract"),
                        String::from(""),
                        String::from("f in not Variable::String"),
                    ));
                }
            }
        };

        if !deep {
            return Ok(extracted);
        }

        // deep extraction.
        extracted.extract(f, deep)
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, str::FromStr};

    use serde_json::Value;

    use super::Variable;

    #[test]
    fn test_extract_non_extractable() {
        let index = Variable::String(String::from("index"));

        let v1 = Variable::None;
        assert!(v1.extract(&index, false).is_err());
        let v1 = Variable::Int(1);
        assert!(v1.extract(&index, false).is_err());
        let v1 = Variable::String(String::from("string"));
        assert!(v1.extract(&index, false).is_err());
        let v1 = Variable::Float(1.0);
        assert!(v1.extract(&index, false).is_err());
        let v1 = Variable::Bool(false);
        assert!(v1.extract(&index, false).is_err());
    }

    #[test]
    fn test_extract_non_extractable_deep() {
        let index = Variable::String(String::from("index"));

        let v1 = Variable::None;
        assert_eq!(v1.extract(&index, true).unwrap(), v1);
        let v1 = Variable::Int(1);
        assert_eq!(v1.extract(&index, true).unwrap(), v1);
        let v1 = Variable::String(String::from("string"));
        assert_eq!(v1.extract(&index, true).unwrap(), v1);
        let v1 = Variable::Float(1.0);
        assert_eq!(v1.extract(&index, true).unwrap(), v1);
        let v1 = Variable::Bool(false);
        assert_eq!(v1.extract(&index, true).unwrap(), v1);
    }

    #[test]
    fn test_extract_vector() {
        let index = Variable::Int(0);
        let index_string = Variable::String(String::from("0"));

        let v2 = Variable::String(String::from("test variable"));

        let v1 = Variable::Vector(vec![v2.clone()]);
        assert_eq!(v1.extract(&index, false).unwrap(), v2);
        assert_eq!(v1.extract(&index, true).unwrap(), v2);
        assert_eq!(v1.extract(&index_string, false).unwrap(), v2);
        assert_eq!(v1.extract(&index_string, true).unwrap(), v2);

        let index = Variable::Int(2);
        let index_string = Variable::String(String::from("2"));
        let longer = Variable::Vector(vec![
            Variable::Int(1),
            Variable::Bool(false),
            v2.clone(),
            Variable::None,
        ]);
        assert_eq!(longer.extract(&index, false).unwrap(), v2);
        assert_eq!(longer.extract(&index, true).unwrap(), v2);
        assert_eq!(longer.extract(&index_string, false).unwrap(), v2);
        assert_eq!(longer.extract(&index_string, true).unwrap(), v2);

        assert!(longer.extract(&Variable::Int(10), false).is_err());
        assert!(longer.extract(&Variable::None, false).is_err());
        assert!(longer.extract(&Variable::Bool(false), false).is_err());
        assert!(longer.extract(&Variable::Float(12.2), false).is_err());
    }

    #[test]
    fn test_extract_vector_deep() {
        let v1 = Variable::Vector(vec![Variable::Vector(vec![Variable::Vector(vec![
            Variable::None,
        ])])]);
        let index = Variable::Int(0);

        assert_eq!(v1.extract(&index, true).unwrap(), Variable::None);

        // vectors are not the same length.
        let v1 = Variable::Vector(vec![
            Variable::None,
            Variable::None,
            Variable::Vector(vec![Variable::None]),
        ]);
        let index = Variable::Int(2);
        assert!(v1.extract(&index, true).is_err());
    }

    #[test]
    fn test_extract_object() {
        let wrong_index = Variable::Int(1);

        let v1 = Variable::Object(HashMap::new());
        assert!(v1.extract(&wrong_index, false).is_err());

        let index = Variable::String(String::from("key2"));
        let v1 = Variable::Object(HashMap::from([
            (String::from("key1"), Variable::None),
            (String::from("key2"), Variable::Int(1)),
        ]));
        assert_eq!(v1.extract(&index, false).unwrap(), Variable::Int(1));
    }

    #[test]
    fn test_extract_object_deep() {
        let index = Variable::String(String::from("key2"));
        let v1 = Variable::Object(HashMap::from([
            (String::from("key1"), Variable::None),
            (
                String::from("key2"),
                Variable::Object(HashMap::from([
                    (String::from("key1"), Variable::None),
                    (
                        String::from("key2"),
                        Variable::Object(HashMap::from([
                            (String::from("key1"), Variable::None),
                            (String::from("key2"), Variable::Int(1)),
                        ])),
                    ),
                ])),
            ),
        ]));
        assert_eq!(v1.extract(&index, true).unwrap(), Variable::Int(1));

        let index = Variable::String(String::from("1"));
        // object -> object -> object -> vector;
        let v1 = Variable::Object(HashMap::from([
            (String::from("key1"), Variable::None),
            (
                String::from("1"),
                Variable::Object(HashMap::from([
                    (String::from("key1"), Variable::None),
                    (
                        String::from("1"),
                        Variable::Object(HashMap::from([
                            (String::from("key1"), Variable::None),
                            (
                                String::from("1"),
                                Variable::Vector(vec![Variable::None, Variable::Int(1)]),
                            ),
                        ])),
                    ),
                ])),
            ),
        ]));
        assert_eq!(v1.extract(&index, true).unwrap(), Variable::Int(1));
    }

    #[test]
    fn test_extract_object_deep_with_nested_json() {
        // object -> object -> object -> json
        let index = Variable::String(String::from("key2"));
        let v1 = Variable::Object(HashMap::from([
            (String::from("key1"), Variable::None),
            (
                String::from("key2"),
                Variable::Object(HashMap::from([
                    (String::from("key1"), Variable::None),
                    (
                        String::from("key2"),
                        Variable::Object(HashMap::from([
                            (String::from("key1"), Variable::None),
                            (
                                String::from("key2"),
                                Variable::Json(
                                    Value::from_str(r#"{"key1": "siema", "key2": 1}"#).unwrap(),
                                ),
                            ),
                        ])),
                    ),
                ])),
            ),
        ]));
        assert_eq!(v1.extract(&index, true).unwrap(), Variable::Int(1));
    }

    #[test]
    fn test_extract_json() {
        let index = Variable::String(String::from("key"));
        let v1 = Variable::Json(Value::from_str(r#"{"key1": "siema", "key": 1}"#).unwrap());

        assert_eq!(v1.extract(&index, false).unwrap(), Variable::Int(1));

        let index = Variable::Int(1);
        assert!(v1.extract(&index, false).is_err());
        let index = Variable::String(String::from("string"));
        assert!(v1.extract(&index, false).is_err());
        let index = Variable::Float(1.0);
        assert!(v1.extract(&index, false).is_err());
        let index = Variable::Bool(false);
        assert!(v1.extract(&index, false).is_err());
    }

    #[test]
    fn test_extract_json_deep() {
        let index = Variable::String(String::from("key"));
        let v1 = Variable::Json(
            Value::from_str(
                r#"
        {
            "key1": "siema",
            "key": {
                "siema": "co tam",
                "nic": 123,
                "spoko": false,
                "key": {
                    "key": {
                        "key": 1
                    }
                }
            }
        }
        "#,
            )
            .unwrap(),
        );
        assert_eq!(v1.extract(&index, true).unwrap(), Variable::Int(1));
    }
}
