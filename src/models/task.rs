use crate::core::types::*;
use crate::{core::task::TrackingTask, schema::tasks};
use diesel::{Insertable, Queryable};

#[derive(Queryable, Insertable)]
#[table_name = "tasks"]
pub struct TaskModel {
    pub uuid: String,           // task id.
    pub name: String,           // task name.
    pub description: String,    // task description.
    pub spreadsheet_id: String, // spreadsheet where data will be written.
    pub position: String,       // starting position of data in the spreadsheet. A1 notation.
    pub sheet: String,          // exact sheet of spreadsheet. Default is empty, first sheet.
    pub direction: Direction,
    pub with_timestamp: bool, // whether to write timestamp.
    pub timestamp_position: TimestampPosition,
    pub process: String,       // definition of handling data.
    pub input: Option<String>, // json of input definition.
    pub status: State,
    pub kind: String, // json of TaskKind definition.
}

impl TaskModel {
    pub fn from_tracking_task(tt: &TrackingTask) -> Self {
        TaskModel {
            uuid: tt.id.to_string(),
            name: tt.name.as_ref().cloned().unwrap_or_default(),
            spreadsheet_id: tt.spreadsheet_id.clone(),
            sheet: tt.sheet.clone(),
            position: tt.starting_position.clone(),
            direction: tt.direction,
            description: tt.description.as_ref().cloned().unwrap_or_default(),
            status: tt.status,
            with_timestamp: tt.with_timestamp,
            timestamp_position: tt.timestamp_position,
            process: tt.process.try_to_string().unwrap_or_default(),
            input: tt.input.as_ref().map(|f| f.to_json()),
            kind: tt.kind_request.to_json(),
        }
    }
}
