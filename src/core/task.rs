use super::types::*;
use crate::connector::factory::getter_from_task_input;
use crate::error::types::{Error, Result};
use crate::lang::lexer::EvalForest;
use crate::models::task::TaskModel;
use crate::server::task::{TaskCreateRequest, TaskKindRequest};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use std::vec::Vec;
use uuid::Uuid;

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub enum TaskInput {
    None,
    String {
        value: String,
    },
    HTTP {
        url: String,
        input_type: InputType,
    },
    PSQL {
        host: String,
        port: u16,
        user: String,
        password: String,
        query: String,
        db: String,
    },
}

impl TaskInput {
    pub fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json).map_err(|err| {
            Error::new_internal(
                String::from("from_string"),
                String::from("failed to deserialize task input"),
                err.to_string(),
            )
        })
    }

    pub fn to_json(&self) -> String {
        serde_json::json!(self).to_string()
    }
}

impl Default for TaskInput {
    fn default() -> Self {
        Self::None
    }
}

/// Enum for user's input data.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub enum InputData {
    String(String),
    Json(Value),
    Vector(Vec<InputData>),
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
    pub with_timestamp: bool, // whether to write timestamp.
    pub timestamp_position: TimestampPosition,
    pub invocations: Option<i32>, // number of invocations.
    pub eval_forest: EvalForest,  // definition of handling data.
    pub status: State,
    pub input: Option<TaskInput>,

    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    // Function that returns data to be written.
    // Can be None because task can get data straight from connector event.
    pub data_fn: Option<Arc<BoxFnThatReturnsAFuture>>,
    pub callbacks: Option<Vec<CallbackFn>>,

    #[derivative(PartialEq = "ignore")]
    pub kind: TaskKind,
    pub kind_request: TaskKindRequest, // request from API, needed for persistance.
}

impl TrackingTask {
    #[allow(clippy::too_many_arguments)]
    // creates new TrackingTask.
    pub fn new(
        spreadsheet_id: String,
        sheet: String,
        starting_position: String,
        direction: Direction,
        data_fn: Option<BoxFnThatReturnsAFuture>,
        kind_request: TaskKindRequest,
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
            data_fn: data_fn.and_then(|f| Some(Arc::new(f))),
            spreadsheet_id,
            sheet,
            starting_position,
            direction,
            callbacks: None,
            with_timestamp: false,
            timestamp_position: TimestampPosition::None,
            invocations: None,
            eval_forest: EvalForest::default(),
            status: State::Created,
            input: None,
            kind: TaskKind::from_task_kind_request(&kind_request),
            kind_request: kind_request,
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

        let input = tm
            .input
            .as_ref()
            .and_then(|i| Some(TaskInput::from_json(&i).unwrap()));
        let kind_request = TaskKindRequest::from_json(&tm.kind)?;

        Ok(TrackingTask {
            id,
            name: Some(tm.name.clone()),
            description: Some(tm.description.clone()),
            data_fn: input
                .as_ref()
                .and_then(|i| getter_from_task_input(&i).and_then(|f| Some(Arc::new(f)))),
            spreadsheet_id: tm.spreadsheet_id.clone(),
            starting_position: tm.position.clone(),
            sheet: tm.sheet.clone(),
            direction: tm.direction,
            with_timestamp: true,
            timestamp_position: tm.timestamp_position,
            invocations: None,
            eval_forest: EvalForest::from_string(&tm.eval_forest)?,
            status: tm.status,
            callbacks: None,
            input,
            kind: TaskKind::from_task_kind_request(&kind_request),
            kind_request,
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

    /// sets eval_forest field.
    pub fn with_eval_forest(mut self, eval_forest: EvalForest) -> TrackingTask {
        self.eval_forest = eval_forest;
        self
    }

    //TODO: refactor - input and data_fn should be highly connected.
    /// sets input field.
    pub fn with_input(mut self, input: TaskInput) -> TrackingTask {
        self.input = Some(input);
        self
    }

    pub fn with_kind(mut self, kind: TaskKind) -> TrackingTask {
        self.kind = kind;
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
        (self.data_fn.as_ref().ok_or(Error::new_internal(
            String::from("TrackingTask:data"),
            String::from("data_fn is None"),
            String::default(),
        ))?)()
        .await
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
        Ok(TrackingTask {
            id: Uuid::new_v4(),
            name: Some(tcr.name),
            description: Some(tcr.description),
            spreadsheet_id: tcr.spreadsheet_id,
            sheet: tcr.sheet,
            starting_position: tcr.starting_position,
            direction: tcr.direction,
            callbacks: None,
            with_timestamp: false,
            timestamp_position: TimestampPosition::None,
            invocations: None,
            eval_forest: EvalForest::from_definition(&tcr.definition),
            data_fn: getter_from_task_input(tcr.input.as_ref().unwrap())
                .and_then(|f| Some(Arc::new(f))),
            status: State::Created,
            input: tcr.input,
            kind: TaskKind::from_task_kind_request(&tcr.kind_request),
            kind_request: tcr.kind_request,
        })
    }
}

mod test {
    #[allow(unused_imports)]
    use crate::core::task::{Direction, InputData, TaskInput, TrackingTask};
    use crate::{error::types::Result, server::task::TaskKindRequest};

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
            Some(Box::new(move || Box::pin(test_get_data_fn()))),
            TaskKindRequest::Ticker { interval_secs: 1 },
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
            Some(Box::new(move || Box::pin(test_get_data_fn()))),
            TaskKindRequest::Ticker { interval_secs: 1 },
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
            Some(Box::new(move || Box::pin(test_get_data_fn()))),
            TaskKindRequest::Ticker { interval_secs: 1 },
        );
        tt = tt.with_description("test".to_string());
        assert_eq!(tt.description.unwrap_or_default().as_str(), "test")
    }

    #[test]
    fn test_task_input_to_json() {
        let ti = TaskInput::PSQL {
            host: String::from("host"),
            port: 5432,
            user: String::from("user"),
            password: String::from("pass"),
            query: String::from("SELECT 1"),
            db: String::from("test"),
        };
        assert_eq!(
            r#"{"PSQL":{"db":"test","host":"host","password":"pass","port":5432,"query":"SELECT 1","user":"user"}}"#,
            ti.to_json()
        )
    }
}
