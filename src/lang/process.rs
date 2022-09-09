use super::engine::Definition;
use serde::{Deserialize, Serialize};

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Highest level of nesting. Contains metadata about whole tree execution.
pub struct Process {
    // name of a whole process.
    pub name: String,

    // set of definitions to run.
    pub definitions: Vec<Definition>,

    // set of mounts to perform.
    pub mounts: Option<Vec<MountOption>>,
}

#[cfg(test)]
mod tests {
    use super::Process;
    use crate::lang::{engine::Definition, process::MountOption};

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
            definitions: vec![Definition {
                steps: vec![
                    String::from("MOCK DEFINITION 1"),
                    String::from("MOCK DEFINITION 2"),
                ],
                subtrees: None,
            }],
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
            definitions: vec![Definition {
                steps: vec![
                    String::from("MOCK DEFINITION 1"),
                    String::from("MOCK DEFINITION 2"),
                ],
                subtrees: None,
            }],
            mounts: Some(vec![MountOption {
                alias: String::from("a"),
                path: String::from("p"),
                mount_type: crate::lang::process::MountType::File,
            }]),
        };

        let process: Process = serde_json::from_str(content).expect("failed to deserialize");
        assert_eq!(wanted, process);
    }
}
