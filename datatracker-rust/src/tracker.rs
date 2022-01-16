use std::vec::Vec;
use uuid;

use crate::wrap::APIWrapper;

// TrackedData is a type wrap for data that is being tracked. It'll be written as string anyway.
type TrackedData = Vec<String>;

// GetDataFn is a type wrap for a function that returns a TrackedData.
type GetDataFn = fn() -> Result<TrackedData, String>;

#[derive(Clone, Debug)]
// Direction indicates direction of written data.
pub enum Direction {
    Vertical,   // data will be written in columns.
    Horizontal, // data will be written in rows.
}

// CallbackFn is a type wrap for callback function.
type CallbackFn = fn(Result<(), String>) -> ();

#[derive(Clone)]
// Callback is a type wrap for a function that will be called when data is written.
pub struct Callback(CallbackFn);

#[derive(Clone)]
// TrackingTask holds information about tracking task.
pub struct TrackingTask {
    id: uuid::Uuid,              // task id.
    name: Option<String>,        // task name.
    description: Option<String>, // task description.

    get_data_fn: GetDataFn, // function that returns data to be written.
    spreadsheet_id: String, // spreadsheet where data will be written.
    // sheet is a exact sheet of spreadsheet. Default is empty, first sheet.
    sheet: String,
    direction: Direction,
    callbacks: Option<Vec<Callback>>,
}

impl TrackingTask {
    // creates new TrackingTask.
    pub fn new(
        spreadsheet_id: String,
        sheet: String,
        direction: Direction,
        get_data_fn: GetDataFn,
    ) -> TrackingTask {
        TrackingTask {
            id: uuid::Uuid::new_v4(),
            name: None,
            description: None,
            get_data_fn,
            spreadsheet_id,
            sheet,
            direction,
            callbacks: None,
        }
    }

    // sets task name.
    pub fn with_name(mut self, name: String) -> TrackingTask {
        self.name = Some(name);
        self
    }

    // sets task description.
    pub fn with_description(mut self, description: String) -> TrackingTask {
        self.description = Some(description);
        self
    }

    // adds callback to task.
    pub fn with_callback(mut self, callback_fn: CallbackFn) -> TrackingTask {
        if self.callbacks.is_none() {
            self.callbacks = Some(Vec::new());
        }
        self.callbacks.as_mut().unwrap().push(Callback(callback_fn));
        self
    }

    // runs task callbacks on result.
    pub fn run_callbacks(&self, result: Result<(), String>) {
        if let Some(callbacks) = &self.callbacks {
            for callback in callbacks {
                callback.0(result.clone());
            }
        }
    }

    pub fn get_data(&self) -> Result<TrackedData, String> {
        (self.get_data_fn)()
    }
}

// Tracker is a wrapper for the Google Sheets API.
// It is used to track various kind of things and keep that data in a Google Sheet.
pub struct Tracker {
    api: APIWrapper,
    tasks: Vec<TrackingTask>,
}

impl Tracker {
    // creates new Tracker.
    pub fn new(api: APIWrapper) -> Tracker {
        Tracker {
            api,
            tasks: Vec::new(),
        }
    }

    // adds new task to Tracker.
    pub fn add_task(&mut self, task: TrackingTask) {
        self.tasks.push(task);
    }

    fn create_write_vec(&self, data: TrackedData) -> Vec<Vec<String>> {
        let mut write_vec = Vec::new();
        match self.tasks[0].direction {
            Direction::Vertical => {
                for v in data {
                    write_vec.push(vec![v]);
                }
            }
            Direction::Horizontal => {
                let mut row = Vec::new();
                for v in data {
                    row.push(v);
                }
                write_vec.push(row);
            }
        }
        write_vec
    }

    // runs all tasks.
    pub async fn run(&mut self) {
        for task in &self.tasks {
            let result = task.get_data();
            match result {
                Ok(data) => {
                    let result = self
                        .api
                        .write(
                            self.create_write_vec(data),
                            &task.spreadsheet_id,
                            &task.sheet,
                        )
                        .await;
                    task.run_callbacks(result);
                }
                Err(e) => {
                    task.run_callbacks(Err(e));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::tracker::TrackedData;

    fn test_get_data_fn() -> Result<TrackedData, String> {
        Ok(vec!["test".to_string()])
    }
    #[test]
    fn callback_test() {
        use crate::tracker::{Direction, TrackingTask};
        let mut tt = TrackingTask::new(
            "".to_string(),
            "A1:B1".to_string(),
            Direction::Vertical,
            test_get_data_fn,
        );
        tt = tt.with_callback(|res: Result<(), String>| {
            assert_eq!(res.is_ok(), true);
        });
        assert!(tt.callbacks.is_some());
        tt.run_callbacks(Ok(()));
    }

    #[test]
    fn task_with_name() {
        use crate::tracker::{Direction, TrackingTask};
        let mut tt = TrackingTask::new(
            "".to_string(),
            "A1:B1".to_string(),
            Direction::Vertical,
            test_get_data_fn,
        );
        tt = tt.with_name("test".to_string());
        assert!(tt.name.is_some());
        assert_eq!(tt.name.unwrap(), "test")
    }

    #[test]
    fn task_with_description() {
        use crate::tracker::{Direction, TrackingTask};
        let mut tt = TrackingTask::new(
            "".to_string(),
            "A1:B1".to_string(),
            Direction::Vertical,
            test_get_data_fn,
        );
        tt = tt.with_description("test".to_string());
        assert!(tt.description.is_some());
        assert_eq!(tt.description.unwrap(), "test")
    }
}
