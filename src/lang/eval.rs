use super::engine::Engine;
use super::variable::Variable;
use super::{engine::Definition, lexer::Node};
use crate::lang::lexer::{Lexer, Parser};
use crate::{
    core::task::InputData,
    error::types::{Error, Result},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Default, Clone, Deserialize, Serialize, PartialEq)]
pub struct EvalForest {
    pub roots: Vec<Node>,
    pub subtrees: HashMap<String, Vec<Node>>,
}

impl EvalForest {
    pub fn from_definition(def: &Definition) -> Self {
        let mut roots = vec![];

        // parse base steps in Definition.
        for step in &def.steps {
            roots.push(Parser::new(Lexer::new(step).make_tokens()).parse().unwrap());
        }

        let mut subtrees = HashMap::default();
        // parse subtrees in Definition.
        for subtree in def.subtrees.as_ref().unwrap_or(&vec![]) {
            let mut roots = vec![];

            // parse base steps in Definition.
            for step in &subtree.definition.steps {
                roots.push(Parser::new(Lexer::new(step).make_tokens()).parse().unwrap());
            }
            subtrees.insert(subtree.name.clone(), roots);
        }

        EvalForest { roots, subtrees }
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
    let mut engine = Engine::new(Variable::from_input_data(data), ef.clone());
    engine.fire()?;
    Ok(engine
        .get("OUT")
        .ok_or_else(|| {
            Error::new_eval_internal(
                String::from("evaluate_data"),
                String::from("There is not OUT variable!!!"),
            )
        })?
        .clone())
}
