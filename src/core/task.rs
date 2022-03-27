use serde::Deserialize;
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;
use std::vec::Vec;
use uuid::Uuid;

use crate::data::getter::getter_from_url;
use crate::lang::engine::Definition;
use crate::web::task::TaskCreateRequest;

/// Supported types for task's input data.
/// Should match with InputData.
#[derive(Debug, Clone, Deserialize, Copy)]
#[serde(rename_all = "lowercase")]
pub enum InputType {
    String,
    Json,
}

/// Enum for user's input data.
#[derive(Debug, Clone, Deserialize)]
pub enum InputData {
    String(String),
    Json(Value),
}

//type aliases added, because this is a chonker of a type
type GetDataResult = Result<InputData, &'static str>;
type BoxFn<T> = Box<dyn Fn() -> T + Send + Sync>;
type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send + Sync>>;

pub type BoxFnThatReturnsAFuture = BoxFn<BoxFuture<GetDataResult>>;

//pub type GetDataFn =
// Box<dyn Fn() -> (dyn Future<Output = Result<InputData, &'static str>> + Send + Sync)>;

// Direction indicates direction of written data.
#[derive(Clone, Debug, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    Vertical,   // data will be written in columns.
    Horizontal, // data will be written in rows.
}

// TimestampPosition indicates position of timestamp in the data.
#[derive(Clone, Debug, Copy, PartialEq)]
pub enum TimestampPosition {
    None, // timestamp will not be written.
    Before,
    After,
}

// CallbackFn is a type wrap for callback function.
type CallbackFn = fn(Result<(), &'static str>) -> ();

// TrackingTask holds information about tracking task.
pub struct TrackingTask {
    id: Uuid,                    // task id.
    name: Option<String>,        // task name.
    description: Option<String>, // task description.

    data_fn: BoxFnThatReturnsAFuture, // function that returns data to be written.
    spreadsheet_id: String,           // spreadsheet where data will be written.
    starting_position: String,        // starting position of data in the spreadsheet. A1 notation.
    sheet: String,                    // exact sheet of spreadsheet. Default is empty, first sheet.
    direction: Direction,
    interval: Duration,   // interval between data writes.
    with_timestamp: bool, // whether to write timestamp.
    timestamp_position: TimestampPosition,
    invocations: Option<i32>, // number of invocations.
    definition: Definition,   // definition of handling data.

    pub callbacks: Option<Vec<CallbackFn>>,
}

impl TrackingTask {
    // creates new TrackingTask.
    pub fn new(
        spreadsheet_id: String,
        sheet: String,
        starting_position: String,
        direction: Direction,
        data_fn: BoxFnThatReturnsAFuture,
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
            data_fn,
            spreadsheet_id,
            sheet,
            starting_position,
            direction,
            interval,
            callbacks: None,
            with_timestamp: false,
            timestamp_position: TimestampPosition::None,
            invocations: None,
            definition: Definition::new(vec![]),
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
                callback(result);
            }
        }
    }

    pub async fn data(&self) -> Result<InputData, &'static str> {
        (self.data_fn)().await
    }

    pub fn name(&self) -> &str {
        if let Some(name) = &self.name {
            name
        } else {
            ""
        }
    }

    pub fn description(&self) -> &str {
        if let Some(name) = &self.description {
            name
        } else {
            ""
        }
    }

    pub fn interval(&self) -> Duration {
        self.interval
    }

    pub fn invocations(&self) -> Option<i32> {
        self.invocations
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn direction(&self) -> Direction {
        self.direction
    }

    pub fn spreadsheet_id(&self) -> String {
        self.spreadsheet_id.clone()
    }

    pub fn sheet(&self) -> String {
        self.sheet.clone()
    }

    pub fn starting_position(&self) -> String {
        self.starting_position.clone()
    }

    pub fn definition(&self) -> &Definition {
        &self.definition
    }

    /// returns String that can be put into logs.
    pub fn info(&self) -> String {
        if let Some(name) = &self.name {
            format!("{}/{}", name.as_str(), self.id.to_simple())
        } else {
            format!("{}", self.id.to_simple())
        }
    }

    pub fn from_task_create_request(tcr: TaskCreateRequest) -> Result<Self, &'static str> {
        let interval = Duration::new(tcr.interval_secs, 0);

        Ok(TrackingTask {
            id: Uuid::new_v4(),
            name: Some(tcr.name),
            description: Some(tcr.description),
            spreadsheet_id: tcr.spreadsheet_id,
            sheet: tcr.sheet,
            starting_position: tcr.starting_position,
            direction: tcr.direction,
            interval,
            callbacks: None,
            with_timestamp: false,
            timestamp_position: TimestampPosition::None,
            invocations: None,
            definition: tcr.definition,
            data_fn: getter_from_url(&tcr.url, tcr.input_type),
        })
    }
}

mod test {
    #[allow(unused_imports)]
    use crate::core::task::{Direction, InputData, TrackingTask};

    #[allow(dead_code)]
    async fn test_get_data_fn() -> Result<InputData, &'static str> {
        Ok(InputData::String(String::from("test")))
    }
    #[test]
    fn callback_test() {
        let mut tt = TrackingTask::new(
            "spreadsheet_id".to_string(),
            "".to_string(),
            "A1:B1".to_string(),
            Direction::Vertical,
            Box::new(move || Box::pin(test_get_data_fn())),
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
            Box::new(move || Box::pin(test_get_data_fn())),
            std::time::Duration::from_secs(1),
        );
        tt = tt.with_name("test".to_string());
        assert!(tt.name() != "");
        assert_eq!(tt.name(), "test")
    }

    #[test]
    fn task_with_description() {
        let mut tt = TrackingTask::new(
            "spreadsheet_id".to_string(),
            "".to_string(),
            "A1:B1".to_string(),
            Direction::Vertical,
            Box::new(move || Box::pin(test_get_data_fn())),
            std::time::Duration::from_secs(1),
        );
        tt = tt.with_description("test".to_string());
        assert_eq!(tt.description(), "test")
    }
}
