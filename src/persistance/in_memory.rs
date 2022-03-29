use crate::core::task::TrackingTask;

use super::interface::Persistance;
use std::collections::HashMap;
use uuid::Uuid;

// InMemoryPersistance implements Persistance for in memory hash map.
pub struct InMemoryPersistance {
    data: HashMap<Uuid, u32>,
    tasks: HashMap<Uuid, TrackingTask>,
}

impl InMemoryPersistance {
    pub fn new() -> Self {
        InMemoryPersistance {
            data: HashMap::new(),
            tasks: HashMap::new(),
        }
    }
}

// default implementation for InMemoryPersistance.
impl Default for InMemoryPersistance {
    fn default() -> Self {
        Self::new()
    }
}

impl Persistance for InMemoryPersistance {
    fn save_location(&mut self, key: Uuid, value: u32) -> Result<(), &'static str> {
        info!("writing: {}{}", key.to_simple(), value);
        self.data.insert(key, value);
        Ok(())
    }

    fn read_location(&self, key: &Uuid) -> Option<u32> {
        for (key, value) in (&self.data).iter() {
            info!("{} / {}", key.to_simple(), value);
        }
        self.data.get(key).copied()
    }

    fn save_task(&mut self, task: &TrackingTask) -> Result<(), String> {
        self.tasks.insert(task.id(), task.clone());
        Ok(())
    }

    fn read_task(&mut self, uuid: Uuid) -> Result<TrackingTask, String> {
        self.tasks
            .get(&uuid)
            .ok_or(String::from("task not found"))
            .map(|x| x.clone())
    }
}
