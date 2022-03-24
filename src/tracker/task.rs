use rand::Rng;
use std::time::Duration;
use std::vec::Vec;
use uuid::Uuid;

// TrackedData is a type wrap for data that is being tracked. It'll be written as string anyway.
pub type TrackedData = Vec<String>;

// GetDataFn is a type wrap for a function that returns a TrackedData.
type GetDataFn = fn() -> Result<TrackedData, &'static str>;

#[derive(Clone, Debug, Copy)]
// Direction indicates direction of written data.
pub enum Direction {
    Vertical,   // data will be written in columns.
    Horizontal, // data will be written in rows.
}
#[derive(Clone, Debug, Copy, PartialEq)]
// TimestampPosition indicates position of timestamp in the data.
pub enum TimestampPosition {
    None, // timestamp will not be written.
    Before,
    After,
}

// CallbackFn is a type wrap for callback function.
type CallbackFn = fn(Result<(), &'static str>) -> ();

#[derive(Clone, Debug)]
// TrackingTask holds information about tracking task.
pub struct TrackingTask {
    id: Uuid,                    // task id.
    name: Option<String>,        // task name.
    description: Option<String>, // task description.

    get_data_fn: GetDataFn,    // function that returns data to be written.
    spreadsheet_id: String,    // spreadsheet where data will be written.
    starting_position: String, // starting position of data in the spreadsheet. A1 notation.
    sheet: String,             // exact sheet of spreadsheet. Default is empty, first sheet.
    direction: Direction,
    interval: Duration,   // interval between data writes.
    with_timestamp: bool, // whether to write timestamp.
    timestamp_position: TimestampPosition,
    invocations: Option<i32>, // number of invocations.

    pub callbacks: Option<Vec<CallbackFn>>,
}

impl TrackingTask {
    // creates new TrackingTask.
    pub fn new(
        spreadsheet_id: String,
        sheet: String,
        starting_position: String,
        direction: Direction,
        get_data_fn: GetDataFn,
        interval: Duration,
    ) -> TrackingTask {
        assert_ne!(spreadsheet_id, "", "spreadsheet_id cannot be empty");
        assert!(
            starting_position.len() >= 2,
            "starting_position must be at least 2 characters long."
        );
        TrackingTask {
            id: Uuid::new_v4(),
            name: None,
            description: None,
            get_data_fn,
            spreadsheet_id,
            sheet,
            starting_position,
            direction,
            interval,
            callbacks: None,
            with_timestamp: false,
            timestamp_position: TimestampPosition::None,
            invocations: None,
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
        self.callbacks.as_mut().unwrap().push(callback_fn);
        self
    }

    // with_timestamp adds timestamp to data.
    pub fn with_timestamp(
        mut self,
        with_timestamp: bool,
        position: TimestampPosition,
    ) -> TrackingTask {
        self.with_timestamp = with_timestamp;
        assert!(
            position != TimestampPosition::None,
            "Timestamp position cannot be None."
        );
        self.timestamp_position = position;
        self
    }

    // with_invocations sets number of invocations.
    pub fn with_invocations(mut self, invocations: i32) -> TrackingTask {
        self.invocations = Some(invocations);
        self
    }

    // runs task callbacks on result.
    pub fn run_callbacks(&self, result: Result<(), &'static str>) {
        info!("running callbacks: {:?}", self.callbacks);
        if let Some(callbacks) = &self.callbacks {
            for callback in callbacks {
                callback(result.clone());
            }
        }
    }

    pub fn get_data(&self) -> Result<TrackedData, &'static str> {
        (self.get_data_fn)()
    }

    pub fn get_name(&self) -> &str {
        if let Some(name) = &self.name {
            name
        } else {
            ""
        }
    }

    pub fn get_description(&self) -> &str {
        if let Some(name) = &self.description {
            name
        } else {
            ""
        }
    }

    pub fn get_interval(&self) -> Duration {
        self.interval
    }

    pub fn get_invocations(&self) -> Option<i32> {
        self.invocations
    }

    pub fn get_id(&self) -> Uuid {
        self.id
    }

    pub fn get_direction(&self) -> Direction {
        self.direction
    }

    pub fn get_spreadsheet_id(&self) -> String {
        self.spreadsheet_id.clone()
    }

    pub fn get_sheet(&self) -> String {
        self.sheet.clone()
    }

    pub fn get_starting_position(&self) -> String {
        self.starting_position.clone()
    }

    /// returns String that can be put into logs.
    pub fn info(&self) -> String {
        if let Some(name) = &self.name {
            format!("{}/{}", name.as_str(), self.id.to_simple())
        } else {
            format!("{}", self.id.to_simple())
        }
    }
}

// random_value_generator generates random values.
pub fn random_value_generator() -> Result<TrackedData, &'static str> {
    let mut rng = rand::thread_rng();
    let mut vec = vec![];

    for _ in 1..10 {
        vec.push(rng.gen_range(0..10).to_string());
    }
    Ok(vec)
}

mod test {
    #[allow(unused_imports)]
    use crate::tracker::task::{Direction, TrackedData, TrackingTask};

    #[allow(dead_code)]
    fn test_get_data_fn() -> Result<TrackedData, &'static str> {
        Ok(vec!["test".to_string()])
    }
    #[test]
    fn callback_test() {
        let mut tt = TrackingTask::new(
            "spreadsheet_id".to_string(),
            "".to_string(),
            "A1:B1".to_string(),
            Direction::Vertical,
            test_get_data_fn,
            std::time::Duration::from_secs(1),
        );
        tt = tt.with_callback(|res: Result<(), &'static str>| {
            assert!(res.is_ok());
        });
        assert!(tt.callbacks.is_some());
        tt.run_callbacks(Ok(()));
    }

    #[test]
    fn task_with_name() {
        let mut tt = TrackingTask::new(
            "spreadsheet_id".to_string(),
            "".to_string(),
            "A1:B1".to_string(),
            Direction::Vertical,
            test_get_data_fn,
            std::time::Duration::from_secs(1),
        );
        tt = tt.with_name("test".to_string());
        assert!(tt.get_name() != "");
        assert_eq!(tt.get_name(), "test")
    }

    #[test]
    fn task_with_description() {
        let mut tt = TrackingTask::new(
            "spreadsheet_id".to_string(),
            "".to_string(),
            "A1:B1".to_string(),
            Direction::Vertical,
            test_get_data_fn,
            std::time::Duration::from_secs(1),
        );
        tt = tt.with_description("test".to_string());
        assert_eq!(tt.get_description(), "test")
    }
}
