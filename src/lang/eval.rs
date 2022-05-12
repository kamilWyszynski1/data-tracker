use super::engine::Engine;
use super::variable::Variable;
use super::{engine::Definition, lexer::Node};
use crate::lang::lexer::{Lexer, Parser};
use crate::{
    core::task::InputData,
    error::types::{Error, Result},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct EvalForest {
    roots: Vec<Node>,
}

impl EvalForest {
    pub fn default() -> Self {
        EvalForest { roots: vec![] }
    }

    pub fn from_definition(def: &Definition) -> Self {
        let mut roots = vec![];
        for step in def.clone().into_iter() {
            assert_eq!(step.matches('(').count(), step.matches(')').count());
            let tokens = Lexer::new(&step).make_tokens();
            let root = Parser::new(tokens).parse();
            roots.push(root);
        }
        EvalForest { roots }
    }

    /// Serializes whole tree to json string.
    pub fn to_string(&self) -> Result<String> {
        serde_json::to_string(self)
            .map_err(|err| Error::new_eval_internal(String::from("to_string"), err.to_string()))
    }

    /// Loads tree from json string.
    pub fn from_string(s: &str) -> Result<Self> {
        serde_json::from_str::<Self>(s)
            .map_err(|err| Error::new_eval_internal(String::from("from_string"), err.to_string()))
    }
}

impl IntoIterator for EvalForest {
    type Item = Node;
    type IntoIter = <Vec<Node> as IntoIterator>::IntoIter; // so that you don't have to write std::vec::IntoIter, which nobody remembers anyway

    fn into_iter(self) -> Self::IntoIter {
        self.roots.into_iter()
    }
}

/// Function creates new engine and calls fire method for given Definition.
pub fn evaluate_data(data: &InputData, ef: &EvalForest) -> Result<Variable> {
    let mut e = Engine::new(Variable::from_input_data(data));
    e.fire(ef)?;
    Ok(e.get(String::from("OUT"))
        .ok_or_else(|| {
            Error::new_eval_internal(
                String::from("evaluate_data"),
                String::from("There is not OUT variable!!!"),
            )
        })?
        .clone())
}
