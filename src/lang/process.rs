use crate::error::types::{Error, Result};
use rocket::data::N;
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
/// Helper definition that can be run inside main tree.
/// IN and OUT type of SubTree is always the same.
pub struct SubTree {
    pub name: String,
    pub definition: Definition,
}

impl SubTree {
    /// Validates if Subtree fields are valid.
    fn validate(&self) -> Result<()> {
        self.definition.validate()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Definition {
    /// Name of a definition.
    pub name: Option<String>,

    /// Steps that are being performed in scope of one Definition.
    pub steps: Vec<String>,

    /// Collection of SubTrees that can be run from steps.
    /// Optionally, Subtrees can be marked to run implicitly.
    pub subtrees: Option<Vec<SubTree>>,

    /// Set of subtrees that will be run implicitly - without need of using 'RunSubtree' command.
    /// Subtrees will be run in a order of initialization.
    /// Implicit subtress will be deleted from 'SharedState' in order to prevent multiple runs.
    pub implicit_subtrees: Option<Vec<String>>,
}

impl Definition {
    pub fn new<S: Into<String>>(steps: Vec<S>) -> Self {
        let steps: Vec<String> = steps.into_iter().map(|s| s.into()).collect();
        for step in &steps {
            assert_eq!(step.matches('(').count(), step.matches(')').count())
        }
        Definition {
            name: None,
            steps,
            subtrees: None,
            implicit_subtrees: None,
        }
    }

    fn name(&self) -> String {
        match &self.name {
            Some(name) => format!("Definition-{name}"),
            None => "Definition".into(),
        }
    }

    /// Validates if Definition fields are valid.
    fn validate(&self) -> Result<()> {
        // steps cannot be empty.
        if self.steps.is_empty() {
            return Err(Error::new_validation(
                self.name().as_str(),
                "Empty steps",
                "steps",
            ));
        }
        if let Some(subtrees) = &self.subtrees {
            // validate nested subtrees.
            for subtree in subtrees {
                subtree.validate()?;
            }
        }
        Ok(())
    }
}

impl IntoIterator for Definition {
    type Item = String;
    type IntoIter = <Vec<String> as IntoIterator>::IntoIter; // so that you don't have to write std::vec::IntoIter, which nobody remembers anyway

    fn into_iter(self) -> Self::IntoIter {
        self.steps.into_iter()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
/// Represents different options for mounting things during process execution.
pub struct MountOption {
    pub alias: String,
    pub path: String,
    pub mount_type: MountType,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]

pub enum MountType {
    File,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
/// Highest level of nesting. Contains metadata about whole tree execution.
pub struct Process {
    // name of a whole process.
    pub name: String,

    // set of definitions to run.
    pub definitions: Vec<Definition>,

    // set of mounts to perform.
    pub mounts: Option<Vec<MountOption>>,
}

impl Process {
    pub fn new<S: Into<String>>(
        name: S,
        definitions: Vec<Definition>,
        mounts: Option<Vec<MountOption>>,
    ) -> Self {
        Self {
            name: name.into(),
            definitions,
            mounts,
        }
    }

    pub fn try_to_string(&self) -> Result<String> {
        serde_json::to_string(self)
            .map_err(|err| Error::new_eval_internal(String::from("to_string"), err.to_string()))
    }

    /// Validates Process struct.
    fn validate(&self) -> Result<()> {
        // definitions cannot be empty.
        if self.definitions.is_empty() {
            return Err(Error::new_validation(
                "Process",
                "Definitions cannot be empty",
                "definitions",
            ));
        }

        // validate definition one by one.
        for definition in &self.definitions {
            definition.validate()?;
        }

        Ok(())
    }
}

impl TryFrom<String> for Process {
    type Error = Error;

    fn try_from(value: String) -> Result<Self> {
        serde_json::from_str::<Self>(&value)
            .map_err(|err| Error::new_eval_internal(String::from("from_string"), err.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::Process;
    use crate::{
        error::types::Error,
        lang::process::{Definition, MountOption},
    };

    #[test]
    fn test_process_deserialize() {
        let content = r#"
        {
            "name": "test process",
            "definitions": [
                {
                    "steps": [
                        "MOCK DEFINITION 1",
                        "MOCK DEFINITION 2"
                    ]
                }
            ]
        }"#;
        let wanted = Process {
            name: String::from("test process"),
            definitions: vec![Definition::new(vec![
                String::from("MOCK DEFINITION 1"),
                String::from("MOCK DEFINITION 2"),
            ])],
            mounts: None,
        };

        let process: Process = serde_json::from_str(content).expect("failed to deserialize");
        assert_eq!(wanted, process);
    }

    #[test]
    fn test_process_deserialize_with_mount() {
        let content = r#"
        {
            "name": "test process",
            "definitions": [
                {
                    "steps": [
                        "MOCK DEFINITION 1",
                        "MOCK DEFINITION 2"
                    ]
                }
            ],
            "mounts": [
                {
                    "alias": "a",
                    "path": "p",
                    "mount_type": "File"
                }
            ]
        }"#;
        let wanted = Process {
            name: String::from("test process"),
            definitions: vec![Definition::new(vec![
                String::from("MOCK DEFINITION 1"),
                String::from("MOCK DEFINITION 2"),
            ])],
            mounts: Some(vec![MountOption {
                alias: String::from("a"),
                path: String::from("p"),
                mount_type: crate::lang::process::MountType::File,
            }]),
        };

        let process: Process = serde_json::from_str(content).expect("failed to deserialize");
        assert_eq!(wanted, process);
    }

    #[test]
    fn test_validate_process() {
        let valid_process = Process {
            name: String::from("test process"),
            definitions: vec![Definition::new(vec!["def1"])],
            mounts: None,
        };
        assert_true!(valid_process.validate().is_ok());

        let invalid_process_empty_definitions = Process {
            name: String::from("test process"),
            definitions: vec![],
            mounts: None,
        };
        assert_eq!(
            invalid_process_empty_definitions.validate(),
            Err(Error::new_validation(
                "Process",
                "Definitions cannot be empty",
                "definitions",
            ))
        )
    }
}
