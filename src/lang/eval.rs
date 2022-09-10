use super::node::Node;
use super::process::Definition;
use crate::error::types::{Error, Result};
use crate::lang::lexer::{Lexer, Parser};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::convert::TryFrom;

#[derive(Debug, Default, Clone, Deserialize, Serialize, PartialEq)]
pub(super) struct EvalForest {
    pub roots: Vec<Node>,
    pub subtrees: HashMap<String, Vec<Node>>,
}

impl EvalForest {
    /// Serializes whole tree to json string.
    pub fn to_string(&self) -> Result<String> {
        serde_json::to_string(self)
            .map_err(|err| Error::new_eval_internal(String::from("to_string"), err.to_string()))
    }
}

impl From<Definition> for EvalForest {
    fn from(def: Definition) -> Self {
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
}

impl TryFrom<&str> for EvalForest {
    type Error = Error;

    /// Loads tree from json string.
    fn try_from(value: &str) -> Result<Self> {
        serde_json::from_str::<Self>(value)
            .map_err(|err| Error::new_eval_internal(String::from("from_string"), err.to_string()))
    }
}

impl TryFrom<String> for EvalForest {
    type Error = Error;

    /// Loads tree from json string.
    fn try_from(value: String) -> Result<Self> {
        serde_json::from_str::<Self>(&value)
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
