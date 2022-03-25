use serde_json::Value;
use std::collections::HashMap;

use crate::tracker::task::InputData;

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
        }
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
    pub fn extract(&self, f: &Variable) -> Result<Variable, &'static str> {
        match *self {
            Variable::None
            | Variable::Bool(_)
            | Variable::Int(_)
            | Variable::Float(_)
            | Variable::String(_) => Err("cannot extract"),
            Variable::Vector(ref vec) => {
                let inx: usize;
                match f {
                    Variable::Int(i) => inx = *i as usize,
                    Variable::String(s) => {
                        inx = s.parse().unwrap();
                    }
                    _ => return Err("invalid index type"),
                }

                let inx = inx as usize;
                if vec.len() < inx - 1 {
                    return Err("index out of range");
                }
                Ok(vec[inx].clone())
            }
            Variable::Object(ref obj) => {
                if let Variable::String(s) = f {
                    match obj.get(s) {
                        Some(v) => Ok(v.clone()),
                        None => Err("json does not have this field"),
                    }
                } else {
                    Err("f in not Variable::String")
                }
            }
            Variable::Json(ref jsn) => {
                if let Variable::String(s) = f {
                    match jsn.get(s) {
                        Some(v) => Ok(serde_value_to_variable(v.clone())),
                        None => Err("json does not have this field"),
                    }
                } else {
                    Err("f in not Variable::String")
                }
            }
        }
    }
}
