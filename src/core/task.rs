use super::types::*;
use crate::data::getter::getter_from_url;
use crate::error::types::{Error, Result};
use crate::lang::lexer::EvalForest;
use crate::models::task::TaskModel;
use crate::web::task::TaskCreateRequest;
use serde::Deserialize;
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use std::vec::Vec;
use uuid::Uuid;

/// Enum for user's input data.
#[derive(Debug, Clone, Deserialize)]
pub enum InputData {
    String(String),
    Json(Value),
}

// type aliases added, because this is a chonker of a type
type GetDataResult = Result<InputData>;
type BoxFn<T> = Box<dyn Fn() -> T + Send + Sync>;
type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send + Sync>>;

pub type BoxFnThatReturnsAFuture = BoxFn<BoxFuture<GetDataResult>>;

// CallbackFn is a type wrap for callback function.
type CallbackFn = fn(Result<()>) -> ();

#[derive(Derivative, Clone)]
#[derivative(Debug, PartialEq)]
// TrackingTask holds information about tracking task.
pub struct TrackingTask {
    pub id: Uuid,                    // task id.
    pub name: Option<String>,        // task name.
    pub description: Option<String>, // task description.
    pub spreadsheet_id: String,      // spreadsheet where data will be written.
    pub starting_position: String,   // starting position of data in the spreadsheet. A1 notation.
    pub sheet: String,               // exact sheet of spreadsheet. Default is empty, first sheet.
    pub direction: Direction,
    pub interval: Duration,   // interval between data writes.
    pub with_timestamp: bool, // whether to write timestamp.
    pub timestamp_position: TimestampPosition,
    pub invocations: Option<i32>, // number of invocations.
    pub eval_forest: EvalForest,  // definition of handling data.
    pub url: String,
    pub input_type: InputType,
    pub status: State,

    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    pub data_fn: Arc<BoxFnThatReturnsAFuture>, // function that returns data to be written.
    pub callbacks: Option<Vec<CallbackFn>>,
}

impl TrackingTask {
    #[allow(clippy::too_many_arguments)]
    // creates new TrackingTask.
    pub fn new(
        spreadsheet_id: String,
        sheet: String,
        starting_position: String,
        direction: Direction,
        data_fn: BoxFnThatReturnsAFuture,
        interval: Duration,
        input_type: InputType,
        url: String,
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
            data_fn: Arc::new(data_fn),
            spreadsheet_id,
            sheet,
            starting_position,
            direction,
            interval,
            callbacks: None,
            with_timestamp: false,
            timestamp_position: TimestampPosition::None,
            invocations: None,
            eval_forest: EvalForest::default(),
            input_type,
            url,
            status: State::Created,
        }
    }

    pub fn from_task_model(tm: &TaskModel) -> Result<Self> {
        let id = Uuid::parse_str(&tm.uuid).map_err(|err| {
            Error::new_internal(
                String::from("from_task_model"),
                String::from("failed to parse uuid"),
                err.to_string(),
            )
        })?;

        Ok(TrackingTask {
            id,
            name: Some(tm.name.clone()),
            description: Some(tm.description.clone()),
            data_fn: Arc::new(getter_from_url(&tm.url, InputType::Json)),
            spreadsheet_id: tm.spreadsheet_id.clone(),
            starting_position: tm.position.clone(),
            sheet: tm.sheet.clone(),
            direction: tm.direction,
            interval: Duration::from_secs(tm.interval_secs as u64),
            with_timestamp: true,
            timestamp_position: tm.timestamp_position,
            invocations: None,
            eval_forest: EvalForest::from_string(&tm.eval_forest)?,
            url: tm.url.clone(),
            input_type: tm.input_type,
            status: tm.status,
            callbacks: None,
        })
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
    pub fn run_callbacks(&self, result: Result<()>) {
        info!("running callbacks: {:?}", self.callbacks);
        if let Some(callbacks) = &self.callbacks {
            for callback in callbacks {
                callback(result.clone());
            }
        }
    }

    pub async fn data(&self) -> Result<InputData> {
        (self.data_fn)().await
    }

    /// returns String that can be put into logs.
    pub fn info(&self) -> String {
        if let Some(name) = &self.name {
            format!("{}/{}", name.as_str(), self.id.to_simple())
        } else {
            format!("{}", self.id.to_simple())
        }
    }

    pub fn from_task_create_request(tcr: TaskCreateRequest) -> Result<Self> {
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
            eval_forest: EvalForest::from_definition(&tcr.definition),
            data_fn: Arc::new(getter_from_url(&tcr.url, tcr.input_type)),
            input_type: tcr.input_type,
            url: tcr.url,
            status: State::Created,
        })
    }
}

mod test {
    #[allow(unused_imports)]
    use crate::core::task::{Direction, InputData, InputType, TrackingTask};
    use crate::error::types::Result;

    #[allow(dead_code)]
    async fn test_get_data_fn() -> Result<InputData> {
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
            InputType::String,
            String::from(""),
        );
        tt = tt.with_callback(|res: Result<()>| {
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
            InputType::String,
            String::from(""),
        );
        tt = tt.with_name("test".to_string());
        assert_eq!(tt.name.unwrap_or_default().as_str(), "test")
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
            InputType::String,
            String::from(""),
        );
        tt = tt.with_description("test".to_string());
        assert_eq!(tt.description.unwrap_or_default().as_str(), "test")
    }
}
