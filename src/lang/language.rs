use super::variable::Variable;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;

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

pub struct State {
    variables: HashMap<String, Variable>,
}

impl State {
    pub fn default() -> Self {
        State {
            variables: HashMap::new(),
        }
    }

    pub fn set(&mut self, key: String, v: Variable) {
        self.variables.insert(key, v);
    }
    pub fn get(&self, key: String) -> &Variable {
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
