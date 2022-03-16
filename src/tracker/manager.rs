use std::collections::HashMap;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use uuid::Uuid; // crate for async traits.

#[derive(Debug, PartialEq)]
/// Command that can be run in Manager.
pub enum Command {
    Resume, // stars stopped task.
    Stop,   // stops task.
    Delete, // delete task.
}

impl Command {
    /// Returns Command based od string value.
    pub fn from_string(s: &str) -> Result<Self, &'static str> {
        match s.to_lowercase().as_str() {
            "resume" => Ok(Command::Resume),
            "stop" => Ok(Command::Stop),
            "delete" => Ok(Command::Delete),
            _ => Err("invalid string"),
        }
    }
}

#[derive(Debug, PartialEq)]
/// Contains tasks' uuid and command to be applied.
pub struct TaskCommand {
    pub id: Uuid,
    pub cmd: Command,
}

impl TaskCommand {
    pub fn new(id: Uuid, cmd: Command) -> Self {
        TaskCommand { id, cmd }
    }
}

#[derive(Clone)]
/// Implements TasksManager.
pub struct SenderManager {
    /// Sender that sends TaskCommands to Tracker that will be distributed to wanted task.
    mapping: HashMap<Uuid, Sender<Command>>,
}

impl SenderManager {
    pub fn default() -> Self {
        SenderManager {
            mapping: HashMap::new(),
        }
    }

    pub fn add_new_mapping(&mut self, uuid: Uuid) -> Receiver<Command> {
        debug!("adding new mapping");
        let (send, receive) = channel::<Command>(1);
        self.mapping.insert(uuid, send);
        receive
    }
    pub async fn apply(&self, uuid: Uuid, cmd: Command) {
        if let Err(err) = self.mapping.get(&uuid).unwrap().send(cmd).await {
            error!(
                "failed to send command to {} task, {:?}",
                uuid.to_simple(),
                err
            )
        };
    }
}
