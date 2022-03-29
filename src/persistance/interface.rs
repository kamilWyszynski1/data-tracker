use crate::core::task::TrackingTask;
use mockall::*;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

pub struct TaskRepresentation {}

#[derive(Clone)]
/// Tracker persistance state shared across all handlers.
pub struct Db {
    shared: Arc<Mutex<Box<dyn Persistance + Send>>>,
}

impl Db {
    /// Create new `Db` instance with given persistance.
    pub fn new(p: Box<dyn Persistance + Send>) -> Self {
        Db {
            shared: Arc::new(Mutex::new(p)),
        }
    }

    pub async fn get(&self, key: &Uuid) -> Option<u32> {
        self.shared.lock().await.read_location(key)
    }

    pub async fn save(&self, key: Uuid, value: u32) -> Result<(), &'static str> {
        self.shared.lock().await.save_location(key, value)
    }

    pub async fn save_task(&mut self, task: &TrackingTask) -> Result<(), String> {
        self.shared.lock().await.save_task(task)
    }
    pub async fn read_task(&mut self, uuid: Uuid) -> Result<TrackingTask, String> {
        self.shared.lock().await.read_task(uuid)
    }
}

#[automock]
// Persistance is a trait for storing info about the current state of tracked data.
pub trait Persistance {
    fn save_location(&mut self, key: Uuid, value: u32) -> Result<(), &'static str>;
    fn read_location(&self, key: &Uuid) -> Option<u32>;
    fn save_task(&mut self, task: &TrackingTask) -> Result<(), String>;
    fn read_task(&mut self, uuid: Uuid) -> Result<TrackingTask, String>;
}
