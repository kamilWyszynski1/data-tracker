use crate::{
    core::{handler::Report, task::TrackingTask, types::State},
    error::types::Error,
    models::report::ReportModel,
};

use super::interface::{PResult, Persistance};
use std::collections::HashMap;
use uuid::Uuid;

// InMemoryPersistance implements Persistance for in memory hash map.
pub struct InMemoryPersistance {
    data: HashMap<Uuid, u32>,
    pub tasks: HashMap<Uuid, TrackingTask>,
    las_report: i32,
    pub reports: HashMap<i32, ReportModel>,
}

impl InMemoryPersistance {
    pub fn new() -> Self {
        InMemoryPersistance::default()
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
        self.tasks.insert(task.id, task.clone());
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
    fn update_task_status(&mut self, uuid: Uuid, status: State) -> PResult<()> {
        self.tasks.entry(uuid).and_modify(|tt| tt.status = status);
        Ok(())
    }

    fn delete_task(&mut self, uuid: Uuid) -> PResult<()> {
        self.tasks.remove(&uuid).ok_or_else(|| {
            Error::new_persistance_internal(
                String::from("there's no task to delete"),
                String::default(),
            )
        })?;
        Ok(())
    }

    fn get_tasks_by_status(&mut self, statuses: &[State]) -> PResult<Vec<TrackingTask>> {
        Ok(self
            .tasks
            .iter()
            .filter(|(_, tt)| statuses.contains(&tt.status))
            .map(|(_, tt)| tt.clone())
            .collect())
    }

    fn save_report(&mut self, report: &Report) -> PResult<i32> {
        self.las_report += 1;
        self.reports
            .insert(self.las_report, ReportModel::from_report(report));
        Ok(self.las_report)
    }
}
