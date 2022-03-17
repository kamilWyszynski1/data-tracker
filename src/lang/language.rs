use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/*

{
    "steps": [
        "DEFINE(data, GET("http://data-url.com"))",
        "DEFINE(date, EXTRACT(date))"
    ]
}
*/

#[derive(Serialize, Deserialize)]
pub struct Definition {
    steps: Vec<String>,
}

impl Definition {
    pub fn new(steps: Vec<String>) -> Self {
        Definition { steps }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum Variable {
    None,
    Bool(bool),
    Int(isize),
    Float(f32),
    String(String),
    Vector(Vec<Variable>),
    Object(HashMap<String, Variable>),
    Json(Value),
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

fn value_object_to_variable_object(v: Value) -> Variable {
    let m: HashMap<String, Variable> = v
        .as_object()
        .unwrap()
        .into_iter()
        .map(|(k, v)| (k.clone(), serde_value_to_variable(v.clone())))
        .collect();
    Variable::Object(m)
}

impl Variable {
    pub fn extract(&self, field_name: &str) -> Result<Variable, &'static str> {
        match *self {
            Variable::None
            | Variable::Bool(_)
            | Variable::Int(_)
            | Variable::Float(_)
            | Variable::String(_)
            | Variable::Vector(_) => Err("cannot extract"),
            Variable::Object(ref obj) => match obj.get(field_name) {
                Some(v) => Ok(v.clone()),
                None => Err("json does not have this field"),
            },
            Variable::Json(ref jsn) => match jsn.get(field_name) {
                Some(v) => Ok(serde_value_to_variable(v.clone())),
                None => Err("json does not have this field"),
            },
        }
    }
}

pub struct State {
    variables: HashMap<String, Variable>,
    vector_split: Regex,
}

impl State {
    fn default() -> Self {
        State {
            variables: HashMap::new(),
            vector_split: Regex::new(r"\,(?![^JSON(]*\))").unwrap(),
        }
    }

    pub fn variable(&self, key: String) -> &Variable {
        self.variables.get(&key).unwrap()
    }

    /// Takes definition run it step by step.
    pub fn fire(&mut self, definition: &Definition) {
        for s in &definition.steps {
            // make sure that all opened braces are closed.
            assert_eq!(s.matches('(').count(), s.matches(')').count());
            self.evaluate(s.to_string());
        }
    }

    /// Evaluates single step recursively.
    fn evaluate(&mut self, step: String) -> Option<Variable> {
        if step.matches('(').count() == 0 {
            dbg!("returning string variable: {}", step.clone());
            return Some(Variable::String(step.replace(' ', "")));
        }
        let split = step.split_once('(');
        let (function, mut rest) = split.unwrap();
        dbg!(rest);
        rest = rest.strip_suffix(')').unwrap();
        match function.to_lowercase().replace(' ', "").as_str() {
            "define" => {
                let (name, arg) = double_argument(rest);
                let value = self.evaluate(arg.to_string()).unwrap(); // evaluate argument.
                self.define(name, value);
                None
            }
            "int" => Some(self.int(rest)),
            "float" => Some(self.float(rest)),
            "json" => Some(self.json(rest)),
            "vec" => {
                let mut values: Vec<Variable> = vec![];
                self.vector_split.split(rest).for_each(|s| {
                    values.push(self.evaluate(s.to_string()).unwrap());
                });
                Some(self.vector(values))
            }
            "object" => Some(self.object(rest)),
            "extract" => {
                let (name, arg) = double_argument(rest);
                Some(self.extract(name, arg.replace(' ', "").as_str()).unwrap())
            }
            _ => None,
        }
    }

    /// Adds defined variable to state.
    fn define(&mut self, variable_name: &str, value: Variable) {
        self.variables.insert(variable_name.to_string(), value);
    }

    /// Returns Variable::Int from &str.
    fn int(&self, value: &str) -> Variable {
        Variable::Int(value.parse().unwrap())
    }

    /// Returns Variable::Float from &str.
    fn float(&self, value: &str) -> Variable {
        Variable::Float(value.parse::<f32>().unwrap())
    }

    /// Returns Variable::Json from &str.
    fn json(&self, value: &str) -> Variable {
        Variable::Json(serde_json::from_str(value).unwrap())
    }

    /// Returns Variable::Array from given values.
    fn vector(&self, values: Vec<Variable>) -> Variable {
        Variable::Vector(values)
    }

    /// Returns Variable::Object from &str.
    fn object(&self, value: &str) -> Variable {
        let v: Value = serde_json::from_str(value).unwrap();
        assert!(v.is_object());
        value_object_to_variable_object(v)
    }

    /// Extracts field from variable.
    fn extract(&self, variable_name: &str, field_name: &str) -> Result<Variable, &'static str> {
        match self.variables.get(variable_name) {
            Some(var) => match var.extract(field_name) {
                Ok(field) => Ok(field),
                Err(err) => Err(err),
            },
            None => Err("variable not defined"),
        }
    }
}

/// Assures that there's only two arguments and returns them.
fn double_argument(s: &str) -> (&str, &str) {
    s.split_once(',').unwrap()
}

#[cfg(test)]
mod tests {
    use crate::lang::language::{Definition, State, Variable};
    use serde_json::{self, Value};
    use std::collections::HashMap;

    /// Runs single test scenario.
    fn test(def: Definition, var_name: String, value: Variable) {
        let mut state = State::default();
        state.fire(&def);
        assert_eq!(*state.variable(var_name), value,);
    }

    // #[test]
    // fn test_parse_define() {
    //     let def = Definition::new(vec!["DEFINE(var, lol)".to_string()]);
    //     test(
    //         def,
    //         "var".to_string(),
    //         Variable::String(String::from("lol")),
    //     );
    // }

    // #[test]
    // fn test_parse_int_define() {
    //     let def = Definition::new(vec!["DEFINE(var, int(2))".to_string()]);
    //     test(def, "var".to_string(), Variable::Int(2));
    // }

    // #[test]
    // fn test_parse_float_define() {
    //     let def = Definition::new(vec!["DEFINE(var, float(2))".to_string()]);
    //     test(def, "var".to_string(), Variable::Float(2.));
    // }

    // #[test]
    // fn test_parse_object_define() {
    //     let map_str = r#"
    //     {
    //         "kid":"kid-value",
    //         "kty":"RSA",
    //         "use":"sig",
    //         "n":"n-value",
    //         "e":"e-value"
    //     }"#;
    //     let mut map: HashMap<String, Variable> = HashMap::new();
    //     map.insert(
    //         String::from("kid"),
    //         Variable::String(String::from("kid-value")),
    //     );
    //     map.insert(String::from("kty"), Variable::String(String::from("RSA")));
    //     map.insert(String::from("use"), Variable::String(String::from("sig")));
    //     map.insert(String::from("n"), Variable::String(String::from("n-value")));
    //     map.insert(String::from("e"), Variable::String(String::from("e-value")));

    //     let mut state = State::default();
    //     let def = Definition::new(vec![
    //         format!("DEFINE(var, object({}))", map_str).to_string(),
    //         String::from("DEFINE(var2, EXTRACT(var, kty))"),
    //         String::from("DEFINE(var3, EXTRACT(var, use))"),
    //         String::from("DEFINE(var4, EXTRACT(var, n))"),
    //     ]);
    //     state.fire(&def);
    //     assert_eq!(*state.variable(String::from("var")), Variable::Object(map));
    //     assert_eq!(
    //         *state.variable(String::from("var2")),
    //         Variable::String(String::from("RSA"))
    //     );
    //     assert_eq!(
    //         *state.variable(String::from("var3")),
    //         Variable::String(String::from("sig"))
    //     );
    //     assert_eq!(
    //         *state.variable(String::from("var4")),
    //         Variable::String(String::from("n-value"))
    //     );
    // }

    // #[test]
    // fn test_parse_object_embedded_define() {
    //     let map_str = r#"
    //     {
    //         "kid":"kid-value",
    //         "kty": {
    //             "use":"sig",
    //             "n":"n-value",
    //             "e":"e-value"
    //         }
    //     }"#;
    //     let mut embedded: HashMap<String, Variable> = HashMap::new();
    //     embedded.insert(String::from("use"), Variable::String(String::from("sig")));
    //     embedded.insert(String::from("n"), Variable::String(String::from("n-value")));
    //     embedded.insert(String::from("e"), Variable::String(String::from("e-value")));
    //     let mut map: HashMap<String, Variable> = HashMap::new();
    //     map.insert(
    //         String::from("kid"),
    //         Variable::String(String::from("kid-value")),
    //     );
    //     let obj = Variable::Object(embedded);
    //     map.insert(String::from("kty"), obj.clone());
    //     let mut state = State::default();
    //     let def = Definition::new(vec![
    //         format!("DEFINE(var, object({}))", map_str).to_string(),
    //         String::from("DEFINE(var2, EXTRACT(var, kty))"),
    //     ]);
    //     state.fire(&def);
    //     assert_eq!(*state.variable(String::from("var")), Variable::Object(map));
    //     assert_eq!(*state.variable(String::from("var2")), obj);
    // }

    // #[test]
    // fn test_parse_json_define() {
    //     let data = r#"
    //     {
    //         "name": "John Doe",
    //         "age": 43,
    //         "phones": [
    //             "+44 1234567",
    //             "+44 2345678"
    //         ]
    //     }"#;
    //     // Parse the string of data into serde_json::Value.
    //     let v: Value = serde_json::from_str(data).unwrap();

    //     let mut state = State::default();
    //     let def = Definition::new(vec![
    //         format!("DEFINE(var, JSON({}))", data).to_string(),
    //         "DEFINE(var2, EXTRACT(var, name))".to_string(),
    //     ]);
    //     state.fire(&def);
    //     assert_eq!(*state.variable("var".to_string()), Variable::Json(v));
    //     assert_eq!(
    //         *state.variable("var2".to_string()),
    //         Variable::String(String::from("John Doe"))
    //     );
    // }
    // #[test]
    // fn test_parse_array_define() {
    //     let def = Definition::new(vec!["DEFINE(var, VEC(1,2,3,4))".to_string()]);
    //     test(
    //         def,
    //         "var".to_string(),
    //         Variable::Vector(vec![
    //             Variable::String(String::from("1")),
    //             Variable::String(String::from("2")),
    //             Variable::String(String::from("3")),
    //             Variable::String(String::from("4")),
    //         ]),
    //     );
    // }

    #[test]
    fn test_parse_array_types_define() {
        let data = r#"
        {
            "name": "John Doe",
            "age": 43,
            "phones": [
                "+44 1234567",
                "+44 2345678"
            ]
        }"#;
        // Parse the string of data into serde_json::Value.
        let v: Value = serde_json::from_str(data).unwrap();
        let def = Definition::new(vec![format!(
            "DEFINE(var, VEC(1, INT(2), FLOAT(3.2), JSON({})))",
            data
        )
        .to_string()]);
        test(
            def,
            "var".to_string(),
            Variable::Vector(vec![
                Variable::String(String::from("1")),
                Variable::Int(2),
                Variable::Float(3.2),
                Variable::Json(v),
            ]),
        );
    }
}
