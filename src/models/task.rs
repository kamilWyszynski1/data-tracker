use crate::{core::task::TrackingTask, schema::tasks};
use diesel::{Insertable, Queryable};

#[derive(Queryable, Insertable)]
#[table_name = "tasks"]
pub struct TaskModel {
    pub uuid: String,           // task id.
    pub name: String,           // task name.
    pub spreadsheet_id: String, // spreadsheet where data will be written.
    pub sheet: String,          // exact sheet of spreadsheet. Default is empty, first sheet.
    pub position: String,       // starting position of data in the spreadsheet. A1 notation.
    pub direction: String,
    pub interval_secs: i32, // interval between data writes.
    pub input_type: String,
    pub url: String,
    pub description: String, // task description.
    pub status: String,
    pub interval: String,
    pub with_timestamp: bool, // whether to write timestamp.
    pub timestamp_position: String,
    pub eval_forest: String, // definition of handling data.
}

impl TaskModel {
    pub fn from_tracking_task(tt: &TrackingTask) -> Self {
        TaskModel {
            uuid: tt.id.to_string(),
            name: tt.name.unwrap_or_default(),
            spreadsheet_id: tt.spreadsheet_id,
            sheet: tt.sheet,
            position: String::from("1"),
            direction: tt.direction.to_string(),
            interval_secs: tt.interval.as_secs() as i32,
            input_type: tt.input_type,
            url: tt.url,
            description: tt.description,
            status: tt.status,
            interval: tt.interval,
            with_timestamp: tt.with_timestamp,
            timestamp_position: tt.timestamp_position,
            eval_forest: tt.eval_forest,
        }
    }
}
