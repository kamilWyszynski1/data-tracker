use crate::{core::task::TrackingTask, error::types::Error};

use super::interface::{PResult, Persistance};
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
    fn save_location(&mut self, key: Uuid, value: u32) -> PResult<()> {
        info!("writing: {}{}", key.to_simple(), value);
        self.data.insert(key, value);
        Ok(())
    }

    fn read_location(&self, key: &Uuid) -> PResult<u32> {
        for (key, value) in (&self.data).iter() {
            info!("{} / {}", key.to_simple(), value);
        }
        Ok(*self.data.get(key).ok_or_else(|| {
            Error::new_persistance_internal(
                String::from("could not find location"),
                String::default(),
            )
        })?)
    }

    fn save_task(&mut self, task: &TrackingTask) -> PResult<()> {
        self.tasks.insert(task.id(), task.clone());
        Ok(())
    }

    fn read_task(&mut self, uuid: Uuid) -> PResult<TrackingTask> {
        self.tasks
            .get(&uuid)
            .ok_or_else(|| {
                Error::new_persistance_internal(
                    String::from("could not find task"),
                    String::default(),
                )
            })
            .map(|x| x.clone())
    }
}
