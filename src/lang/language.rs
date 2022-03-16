use serde::{Deserialize, Serialize};
use serde_json::json;
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
    Array(Vec<Variable>),
    Object(HashMap<String, Variable>),
    Json(Value),
}

impl Variable {
    fn new_json(v: Value) -> Variable {
        Variable::Json(v)
    }

    fn extract(&self, field_name: &str) -> Result<Variable, &'static str> {
        match *self {
            Variable::None
            | Variable::Bool(_)
            | Variable::Int(_)
            | Variable::Float(_)
            | Variable::String(_)
            | Variable::Array(_) => Err("cannot extract"),
            Variable::Object(ref obj) => match obj.get(field_name) {
                Some(v) => return Ok(v.clone()),
                None => return Err("json does not have this field"),
            },
            Variable::Json(ref jsn) => match jsn.get(field_name) {
                Some(v) => return Ok(Variable::new_json(v.clone())),
                None => return Err("json does not have this field"),
            },
        }
    }
}

pub struct State {
    variables: HashMap<String, Variable>,
}

impl State {
    fn default() -> Self {
        State {
            variables: HashMap::new(),
        }
    }

    pub fn variable(&self, key: String) -> &Variable {
        self.variables.get(&key).unwrap()
    }

    /// Takes definition run it step by step.
    pub fn fire(&mut self, definition: &Definition) {
        for step in &definition.steps {
            self.parse(step.to_string())
        }
    }

    /// Takes single step and parses it.
    fn parse(&mut self, step: String) {
        let mut split = step.split('(');
        let (function, rest) = (
            split.next().unwrap(),
            split.next().unwrap().trim_end_matches(')'), // delete closing ')'.
        );
        match function.to_lowercase().as_str() {
            "define" => {
                let mut split = rest.split(",");
                let (variable_name, value) = (
                    split.next().unwrap().replace(" ", ""),
                    split.next().unwrap().replace(" ", ""),
                );
                // by default it variable value is parsed to string.
                self.variables
                    .insert(variable_name, Variable::String(value));
            }
            "json" => {}
            "extract" => {}
            _ => {}
        }
    }

    fn define(&mut self, variable_name: &str, value: Variable) {
        self.variables.insert(variable_name.to_string(), value);
    }

    fn json(&self, value: &str) -> Variable {
        Variable::Json(json!(value))
    }

    fn extract(&self, variable_name: &str, field_name: &str) -> Variable {
        // match self.variables.get(variable_name) {
        //     Some(v) => match v.extract(field_name) {
        //         Ok(v) => Ok(*v),
        //         Err(err) => Err(err),
        //     },
        //     None => Err("cannot find variable extract"),
        // }
        self.variables
            .get(variable_name)
            .unwrap()
            .extract(field_name)
            .unwrap()
    }
}

#[cfg(test)]
mod tests {
    use crate::lang::language::{Definition, State, Variable};

    #[test]
    fn test_parse_define() {
        let mut state = State::default();
        let def = Definition::new(vec!["DEFINE(var, lol)".to_string()]);
        state.fire(&def);
        assert_eq!(
            *state.variable(String::from("var")),
            Variable::String(String::from("lol"))
        )
    }
}
