use super::{
    eval::EvalForest,
    node::{EvalMetadata, SharedState},
    process::{MountOption, MountType, Process},
    variable::Variable,
};
use crate::error::types::Result;
use serde::{Deserialize, Serialize};
use std::{
    cell::RefCell,
    collections::HashMap,
    fs::File,
    io::{BufReader, Read},
    rc::Rc,
};

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
    // common state for every definition.
    variables: HashMap<String, Variable>,

    //TODO: refactor and delete.
    eval_forest: EvalForest,

    // set of eval forest to run.
    eval_forests: Vec<EvalForest>,

    // set of mounted readers.
    mounted: HashMap<String, Rc<RefCell<dyn Read>>>,
}

impl Engine {
    pub fn default() -> Self {
        Engine {
            variables: HashMap::new(),
            eval_forest: EvalForest::default(),
            eval_forests: vec![],
            mounted: HashMap::new(),
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
            eval_forests: vec![],
            mounted: HashMap::new(),
        }
    }

    pub fn new2(in_var: Variable, process: Process) -> Result<Self> {
        let mounted = mount_options(&process.mounts.unwrap_or_default())?;

        let mut variables = HashMap::new();
        variables.insert(String::from("IN"), in_var.clone());
        variables.insert(String::from("OUT"), in_var);

        let mut efs = vec![];
        // parse Process into Vec of EvalForest.

        for def in process.definitions {
            efs.push(EvalForest::from_definition(&def));
        }

        Ok(Self {
            variables,
            eval_forest: EvalForest::default(),
            eval_forests: efs,
            mounted,
        })
    }

    pub fn get(&self, key: &str) -> Option<&Variable> {
        self.variables.get(key)
    }

    /// Takes definition run it step by step.
    pub fn fire(&mut self) -> Result<()> {
        let mut shared_state = SharedState {
            variables: self.variables.clone(),
            subtress: self.eval_forest.subtrees.clone(),
            mounted: HashMap::new(),
            eval_metadata: EvalMetadata::default(),
        };

        for root in self.eval_forest.roots.clone().into_iter() {
            root.start_evaluation(&mut shared_state)?;
        }

        // rewrite variables from tree execution.
        self.variables = shared_state.variables;
        Ok(())
    }

    /// Takes set of eval forest and runs them one by one.
    pub fn fire2(&mut self) -> Result<()> {
        let mut shared_state = SharedState {
            variables: self.variables.clone(),
            subtress: HashMap::default(),
            mounted: self.mounted.clone(),
            eval_metadata: EvalMetadata::default(),
        };

        for ef in self.eval_forests.clone() {
            for root in ef.clone() {
                shared_state.subtress = ef.subtrees.clone();
                root.start_evaluation(&mut shared_state)?;
            }
        }

        // rewrite variables from tree execution.
        self.variables = shared_state.variables;

        Ok(())
    }
}

fn mount_options(options: &[MountOption]) -> Result<HashMap<String, Rc<RefCell<dyn Read>>>> {
    let mut mounted: HashMap<String, Rc<RefCell<dyn Read>>> = HashMap::new();

    for opt in options {
        match opt.mount_type {
            MountType::File => {
                let file = File::open(&opt.path)?;
                let reader = BufReader::new(file);

                mounted.insert(opt.alias.clone(), Rc::new(RefCell::new(reader)));
            }
        }
    }

    Ok(mounted)
}

#[cfg(test)]
mod tests {
    use super::Engine;
    use crate::error::types::Result;
    use crate::lang::process::Process;
    use crate::lang::variable::Variable;
    use anyhow::Context;
    use std::fs::File;
    use std::io::BufReader;
    use std::path::Path;

    fn process_file_to_struct(path: impl AsRef<Path>) -> Result<Process> {
        let file = File::open(path.as_ref()).context(format!(
            "could not open file: {}",
            path.as_ref().to_str().unwrap()
        ))?;
        let reader = BufReader::new(file);

        // Read the JSON contents of the file as an instance of `User`.
        let u = serde_json::from_reader(reader).context("could not deserialize to process")?;

        // Return the `User`.
        Ok(u)
    }

    #[test]
    fn test_process1() {
        let path = std::env::current_dir()
            .unwrap()
            .join("src")
            .join("lang")
            .join("test_data")
            .join("process1.json");

        let process = process_file_to_struct(path).unwrap();

        let mut engine = Engine::new2(Variable::None, process).unwrap();
        engine.fire2().unwrap();

        assert_eq!(
            engine.get("OUT").unwrap(),
            &Variable::String(String::from("test data here"))
        );
    }
}
