use super::engine::Definition;

/// Highest level of nesting. Contains metadata about whole tree execution.
pub struct Process {
    // name of a whole process.
    name: String,

    // set of definitions to run.
    definitions: Vec<Definition>,
}

/// Represents different options for mounting things during process execution.
pub enum MountOption {
    /// Mount file content with given `alias`.
    File(String, String),
}
