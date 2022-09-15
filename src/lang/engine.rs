use super::{
    eval::EvalForest,
    node::{EvalMetadata, SharedState},
    process::{MountOption, MountType, Process},
    variable::Variable,
};
use crate::error::types::Result;
use std::{
    cell::RefCell,
    collections::HashMap,
    fs::File,
    io::{BufReader, Read},
    rc::Rc,
};

pub struct Engine {
    // common state for every definition.
    variables: HashMap<String, Variable>,

    // set of eval forest to run.
    eval_forests: Vec<EvalForest>,

    // set of mounted readers.
    mounted: HashMap<String, Rc<RefCell<dyn Read>>>,
}

impl Engine {
    pub fn default() -> Self {
        Engine {
            variables: HashMap::new(),
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
    pub fn new(in_var: Variable, process: Process) -> Result<Self> {
        let mounted = mount_options(&process.mounts.unwrap_or_default())?;

        let mut variables = HashMap::new();
        variables.insert(String::from("IN"), in_var.clone());
        variables.insert(String::from("OUT"), in_var);

        let mut eval_forests = vec![];
        // parse Process into Vec of EvalForest.

        for def in process.definitions {
            eval_forests.push(EvalForest::from(def));
        }

        Ok(Self {
            variables,
            eval_forests,
            mounted,
        })
    }

    pub fn get(&self, key: &str) -> Option<&Variable> {
        self.variables.get(key)
    }

    /// Takes set of eval forest and runs them one by one.
    pub fn fire(&mut self) -> Result<()> {
        let mut shared_state = SharedState::new_with_mounted(
            self.variables.clone(),
            HashMap::new(),
            self.mounted.clone(),
        );

        for ef in self.eval_forests.clone() {
            for root in ef.clone() {
                shared_state.subtress = ef.subtrees.clone();
                root.start_evaluation(&mut shared_state)?;
            }

            // for now we only support 1 level of nesting.
            for (subtree_name, roots) in &ef.implicit_subtrees {
                debug!("implicitly running {subtree_name} subtree");
                for root in roots {
                    root.start_evaluation(&mut shared_state)?;
                }
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

        let u = serde_json::from_reader(reader).context("could not deserialize to process")?;

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

        let mut engine = Engine::new(Variable::None, process).unwrap();
        engine.fire().unwrap();

        assert_eq!(
            engine.get("OUT").unwrap(),
            &Variable::String(String::from("test data here"))
        );
    }

    #[test]
    fn test_process2() {
        env_logger::try_init();

        let path = std::env::current_dir()
            .unwrap()
            .join("src")
            .join("lang")
            .join("test_data")
            .join("process2.json");

        let process = process_file_to_struct(path).unwrap();
        let mut engine = Engine::new(Variable::None, process).unwrap();
        engine.fire().unwrap();

        assert_eq!(
            engine.get("OUT").unwrap(),
            &Variable::Vector(vec![
                Variable::String("test data here".into()),
                Variable::String("test file content2".into())
            ])
        );
    }

    #[test]
    fn test_process3() {
        env_logger::try_init();

        let path = std::env::current_dir()
            .unwrap()
            .join("src")
            .join("lang")
            .join("test_data")
            .join("process3.json");

        let process = process_file_to_struct(path).unwrap();
        let mut engine = Engine::new(Variable::String("".into()), process).unwrap();
        engine.fire().unwrap();

        assert_eq!(engine.get("OUT").unwrap(), &Variable::Int(13));
    }
}
