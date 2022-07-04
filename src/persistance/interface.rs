use crate::{
    core::{handler::Report, task::TrackingTask, types::State},
    error::types::Error,
};
use mockall::*;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

pub type PResult<T> = Result<T, Error>;

#[automock]
// Persistance is a trait for storing info about the current state of tracked data.
pub trait Persistance {
    fn save_location(&mut self, key: Uuid, value: u32) -> PResult<()>;
    fn read_location(&self, key: &Uuid) -> PResult<u32>;
    fn save_task(&mut self, task: &TrackingTask) -> PResult<()>;
    fn read_task(&mut self, uuid: Uuid) -> PResult<TrackingTask>;
    fn update_task_status(&mut self, uuid: Uuid, status: State) -> PResult<()>;
    fn delete_task(&mut self, uuid: Uuid) -> PResult<()>;
    fn get_tasks_by_status(&mut self, statuses: &[State]) -> PResult<Vec<TrackingTask>>;
    fn save_report(&mut self, report: &Report) -> PResult<i32>;
}

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
    pub async fn get(&self, key: &Uuid) -> PResult<u32> {
        self.shared.lock().await.read_location(key)
    }
    pub async fn save(&self, key: Uuid, value: u32) -> PResult<()> {
        self.shared.lock().await.save_location(key, value)
    }
    pub async fn save_task(&mut self, task: &TrackingTask) -> PResult<()> {
        self.shared.lock().await.save_task(task)
    }
    pub async fn read_task(&mut self, uuid: Uuid) -> PResult<TrackingTask> {
        self.shared.lock().await.read_task(uuid)
    }
    pub async fn update_task_status(&mut self, uuid: Uuid, status: State) -> PResult<()> {
        self.shared.lock().await.update_task_status(uuid, status)
    }
    pub async fn delete_task(&mut self, uuid: Uuid) -> PResult<()> {
        self.shared.lock().await.delete_task(uuid)
    }
    pub async fn get_tasks_by_status(&mut self, statuses: &[State]) -> PResult<Vec<TrackingTask>> {
        self.shared.lock().await.get_tasks_by_status(statuses)
    }
    pub async fn save_report(&mut self, report: &Report) -> PResult<i32> {
        self.shared.lock().await.save_report(report)
    }
}
