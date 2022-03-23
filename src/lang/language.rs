use super::variable::Variable;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;

#[derive(Serialize, Deserialize)]
pub struct Definition {
    steps: Vec<String>,
}

impl Definition {
    pub fn new(steps: Vec<String>) -> Self {
        Definition { steps }
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
    pub fn fire(&mut self, definition: &Definition) -> Result<(), Box<dyn Error>> {
        for s in &definition.steps {
            // make sure that all opened braces are closed.
            assert_eq!(s.matches('(').count(), s.matches(')').count());
        }
        Ok(())
    }
}

// #[cfg(test)]
// mod tests {
//     use crate::lang::language::{Definition, State, Variable};
//     use serde_json::{self, Value};
//     use std::collections::HashMap;

//     /// Runs single test scenario.
//     fn test(def: Definition, var_name: String, value: Variable) {
//         let mut state = State::default();
//         state.fire(&def);
//         assert_eq!(*state.variable(var_name), value,);
//     }

//     fn test_err(def: Definition, err: String) {
//         let mut state = State::default();
//         let f = state.fire(&def);
//         assert!(f.is_err());
//         assert_eq!(f.unwrap_err().to_string(), err);
//     }

//     #[test]
//     fn test_parse_define() {
//         let def = Definition::new(vec!["DEFINE(var, lol)".to_string()]);
//         test(
//             def,
//             "var".to_string(),
//             Variable::String(String::from("lol")),
//         );
//     }

//     #[test]
//     fn test_parse_int_define() {
//         let def = Definition::new(vec!["DEFINE(var, int(2))".to_string()]);
//         test(def, "var".to_string(), Variable::Int(2));
//     }

//     #[test]
//     fn test_parse_float_define() {
//         let def = Definition::new(vec!["DEFINE(var, float(2))".to_string()]);
//         test(def, "var".to_string(), Variable::Float(2.));
//     }

//     #[test]
//     fn test_parse_object_define() {
//         let map_str = r#"
//         {
//             "kid":"kid-value",
//             "kty":"RSA",
//             "use":"sig",
//             "n":"n-value",
//             "e":"e-value"
//         }"#;
//         let mut map: HashMap<String, Variable> = HashMap::new();
//         map.insert(
//             String::from("kid"),
//             Variable::String(String::from("kid-value")),
//         );
//         map.insert(String::from("kty"), Variable::String(String::from("RSA")));
//         map.insert(String::from("use"), Variable::String(String::from("sig")));
//         map.insert(String::from("n"), Variable::String(String::from("n-value")));
//         map.insert(String::from("e"), Variable::String(String::from("e-value")));

//         let mut state = State::default();
//         let def = Definition::new(vec![
//             format!("DEFINE(var, object({}))", map_str).to_string(),
//             String::from("DEFINE(var2, EXTRACT(var, kty))"),
//             String::from("DEFINE(var3, EXTRACT(var, use))"),
//             String::from("DEFINE(var4, EXTRACT(var, n))"),
//         ]);
//         state.fire(&def);
//         assert_eq!(*state.variable(String::from("var")), Variable::Object(map));
//         assert_eq!(
//             *state.variable(String::from("var2")),
//             Variable::String(String::from("RSA"))
//         );
//         assert_eq!(
//             *state.variable(String::from("var3")),
//             Variable::String(String::from("sig"))
//         );
//         assert_eq!(
//             *state.variable(String::from("var4")),
//             Variable::String(String::from("n-value"))
//         );
//     }

//     #[test]
//     fn test_parse_object_embedded_define() {
//         let map_str = r#"
//         {
//             "kid":"kid-value",
//             "kty": {
//                 "use":"sig",
//                 "n":"n-value",
//                 "e":"e-value"
//             }
//         }"#;
//         let mut embedded: HashMap<String, Variable> = HashMap::new();
//         embedded.insert(String::from("use"), Variable::String(String::from("sig")));
//         embedded.insert(String::from("n"), Variable::String(String::from("n-value")));
//         embedded.insert(String::from("e"), Variable::String(String::from("e-value")));
//         let mut map: HashMap<String, Variable> = HashMap::new();
//         map.insert(
//             String::from("kid"),
//             Variable::String(String::from("kid-value")),
//         );
//         let obj = Variable::Object(embedded);
//         map.insert(String::from("kty"), obj.clone());
//         let mut state = State::default();
//         let def = Definition::new(vec![
//             format!("DEFINE(var, object({}))", map_str).to_string(),
//             String::from("DEFINE(var2, EXTRACT(var, kty))"),
//         ]);
//         state.fire(&def);
//         assert_eq!(*state.variable(String::from("var")), Variable::Object(map));
//         assert_eq!(*state.variable(String::from("var2")), obj);
//     }

//     #[test]
//     fn test_parse_json_define() {
//         let data = r#"
//         {
//             "name": "John Doe",
//             "age": 43,
//             "phones": [
//                 "+44 1234567",
//                 "+44 2345678"
//             ]
//         }"#;
//         // Parse the string of data into serde_json::Value.
//         let v: Value = serde_json::from_str(data).unwrap();

//         let mut state = State::default();
//         let def = Definition::new(vec![
//             format!("DEFINE(var, JSON({}))", data).to_string(),
//             "DEFINE(var2, EXTRACT(var, name))".to_string(),
//         ]);
//         state.fire(&def);
//         assert_eq!(*state.variable("var".to_string()), Variable::Json(v));
//         assert_eq!(
//             *state.variable("var2".to_string()),
//             Variable::String(String::from("John Doe"))
//         );
//     }

//     #[test]
//     fn test_parse_array_define() {
//         let def = Definition::new(vec!["DEFINE(var, VEC(1,2,3,4))".to_string()]);
//         test(
//             def,
//             "var".to_string(),
//             Variable::Vector(vec![
//                 Variable::String(String::from("1")),
//                 Variable::String(String::from("2")),
//                 Variable::String(String::from("3")),
//                 Variable::String(String::from("4")),
//             ]),
//         );
//     }

//     #[test]
//     fn test_parse_array_types_define() {
//         let data = r#"
//         {
//             "name": "John Doe",
//             "age": 43,
//             "phones": [
//                 "+44 1234567",
//                 "+44 2345678"
//             ]
//         }"#;
//         // Parse the string of data into serde_json::Value.
//         let v: Value = serde_json::from_str(data).unwrap();
//         let def = Definition::new(vec![format!(
//             "DEFINE(var, VEC(1, INT(2), FLOAT(3.2), JSON({})))",
//             data
//         )
//         .to_string()]);
//         test(
//             def,
//             "var".to_string(),
//             Variable::Vector(vec![
//                 Variable::String(String::from("1")),
//                 Variable::Int(2),
//                 Variable::Float(3.2),
//                 Variable::Json(v),
//             ]),
//         );
//     }

//     #[test]
//     fn test_parse_array_define_extract() {
//         let def = Definition::new(vec![
//             String::from("DEFINE(var, VEC(1,2,3,4))"),
//             String::from("DEFINE(var2, EXTRACT(var, 3))"),
//         ]);
//         test(def, "var2".to_string(), Variable::String(String::from("4")));
//     }

//     #[test]
//     fn test_parse_get_define() {
//         let data = r#"{
//             "userId": 1,
//             "id": 1,
//             "title": "delectus aut autem",
//             "completed": false
//         }"#;
//         let v: Value = serde_json::from_str(data).unwrap();

//         let def = Definition::new(vec![String::from(
//             "DEFINE(var, GET(https://jsonplaceholder.typicode.com/todos/1))",
//         )]);
//         test(def, "var".to_string(), Variable::Json(v));
//     }

//     #[test]
//     fn test_parse_get_define_extract() {
//         let def = Definition::new(vec![
//             String::from("DEFINE(var, GET(https://jsonplaceholder.typicode.com/todos/1))"),
//             String::from("DEFINE(var2, EXTRACT(var, title))"),
//         ]);
//         test(
//             def,
//             "var2".to_string(),
//             Variable::String(String::from("delectus aut autem")),
//         );
//     }
// }
