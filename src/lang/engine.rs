use super::lexer::{Lexer, Parser};
use super::variable::Variable;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Definition {
    steps: Vec<String>,
}

impl Definition {
    pub fn new(steps: Vec<String>) -> Self {
        Definition { steps }
    }
}

impl IntoIterator for Definition {
    type Item = String;
    type IntoIter = <Vec<String> as IntoIterator>::IntoIter; // so that you don't have to write std::vec::IntoIter, which nobody remembers anyway

    fn into_iter(self) -> Self::IntoIter {
        self.steps.into_iter()
    }
}

pub struct Engine {
    variables: HashMap<String, Variable>,
}

impl Engine {
    pub fn default() -> Self {
        Engine {
            variables: HashMap::new(),
        }
    }

    /// Creates new instance of Engine with IN and OUT variables set.
    ///
    /// IN variable is a entry variable that is being set
    /// at the very begging, it's a input-like for whole engine.
    ///
    /// OUT variable is a variable that will be a result as
    /// a evaluation that will happen in engine. User should
    /// write wanted data to OUT at last as this variable will
    /// be taken out from Engine after all.
    pub fn new(in_var: Variable) -> Self {
        let mut variables = HashMap::new();
        variables.insert(String::from("IN"), in_var.clone());
        variables.insert(String::from("OUT"), in_var);
        Engine { variables }
    }

    pub fn set(&mut self, key: String, v: Variable) {
        self.variables.insert(key, v);
    }
    pub fn get(&self, key: String) -> &Variable {
        self.variables.get(&key).unwrap()
    }

    /// Takes definition run it step by step.
    pub fn fire(&mut self, definition: &Definition) -> Result<(), &'static str> {
        for s in &definition.steps {
            // make sure that all opened braces are closed.
            assert_eq!(s.matches('(').count(), s.matches(')').count());
            let root = Parser::new(Lexer::new(s).make_tokens()).parse();
            root.eval(self);
        }
        Ok(())
    }
}
