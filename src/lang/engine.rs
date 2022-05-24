use super::{
    eval::EvalForest,
    lexer::{Node, Stack},
    variable::Variable,
};
use crate::error::types::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
/// Helper definition that can be run inside main tree.
/// IN and OUT type of SubTree is always the same.
pub struct SubTree {
    pub name: String,
    pub input_type: Option<String>,
    pub definition: Definition,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Definition {
    pub steps: Vec<String>,
    pub subtrees: Option<Vec<SubTree>>,
}

impl Definition {
    pub fn new(steps: Vec<String>) -> Self {
        for step in &steps {
            assert_eq!(step.matches('(').count(), step.matches(')').count())
        }
        Definition {
            steps,
            subtrees: None,
        }
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
    eval_forest: EvalForest,
}

impl Engine {
    pub fn default() -> Self {
        Engine {
            variables: HashMap::new(),
            eval_forest: EvalForest::default(),
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
    pub fn new(in_var: Variable, eval_forest: EvalForest) -> Self {
        let mut variables = HashMap::new();
        variables.insert(String::from("IN"), in_var.clone());
        variables.insert(String::from("OUT"), in_var);
        Engine {
            variables,
            eval_forest,
        }
    }

    pub fn new_for_subtree(
        in_var: Variable,
        mut variables: HashMap<String, Variable>,
        eval_forest: EvalForest,
    ) -> Self {
        variables.insert(String::from("IN"), in_var.clone());
        variables.insert(String::from("OUT"), in_var);
        Engine {
            variables,
            eval_forest,
        }
    }

    pub fn set(&mut self, key: String, v: Variable) {
        self.variables.insert(key, v);
    }
    pub fn get(&self, key: &str) -> Option<&Variable> {
        self.variables.get(key)
    }

    /// Takes definition run it step by step.
    pub fn fire(&mut self) -> Result<()> {
        for root in self.eval_forest.roots.clone().into_iter() {
            root.start_evaluation(&mut self.variables, &self.eval_forest.subtrees)?;
        }
        Ok(())
    }
}

pub fn evaluate(in_var: Option<Variable>, eval_forest: &EvalForest) -> Result<Variable> {
    let mut variables = HashMap::new();

    in_var.and_then(|variable| {
        variables.insert(String::from("IN"), variable.clone());
        variables.insert(String::from("OUT"), variable);
        Some(())
    });

    fire(&eval_forest.roots, &mut variables, &eval_forest.subtrees)?;

    variables
        .get("OUT")
        .ok_or(Error::new_eval_internal(
            String::from("evaluate"),
            String::from("failed to get 'OUT' variable"),
        ))
        .and_then(|v| Ok(v.clone()))
}

pub fn fire(
    roots: &[Node],
    variables: &mut HashMap<String, Variable>,
    subtrees: &HashMap<String, Vec<Node>>,
) -> Result<()> {
    for root in roots {
        root.start_evaluation(variables, subtrees)?;
    }
    Ok(())
}
