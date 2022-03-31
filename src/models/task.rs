use crate::core::direction::Direction;
use crate::core::intype::InputType;
use crate::core::timestamp::TimestampPosition;
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
    pub interval_secs: i32,   // interval between data writes.
    pub with_timestamp: bool, // whether to write timestamp.
    pub timestamp_position: TimestampPosition,
    pub eval_forest: String, // definition of handling data.
    pub url: String,
    pub input_type: InputType,
    pub status: String,
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
            interval_secs: tt.interval.as_secs() as i32,
            input_type: tt.input_type,
            url: tt.url.clone(),
            description: tt.description.as_ref().cloned().unwrap_or_default(),
            status: String::from("OK"),
            with_timestamp: tt.with_timestamp,
            timestamp_position: tt.timestamp_position,
            eval_forest: tt.eval_forest.to_string().unwrap_or_default(),
        }
    }
}
