use super::channels::ChannelsManager;
use super::types::*;
use crate::connector::factory::getter_from_task_input;
use crate::connector::kafka::{consume_topic, KafkaConfig};
use crate::connector::psql::{monitor_changes, PSQLConfig};
use crate::error::types::{Error, Result};
use crate::lang::process::Process;
use crate::models::task::TaskModel;
use crate::server::task::{TaskCreateRequest, TaskKindRequest};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::convert::TryFrom;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use std::vec::Vec;
use tokio::sync::mpsc::channel as create_channel;
use tokio::sync::{broadcast, Mutex};
use tokio::task::JoinHandle;
use uuid::Uuid;

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum InputData {
    String(String),
    Json(Value),
    Vector(Vec<InputData>),
}

impl InputData {
    pub fn to_str(&self) -> Result<String> {
        serde_json::to_string(self).map_err(|e| {
            Error::new_internal(
                String::from("InputData::to_str"),
                String::from("failed to serialize"),
                e.to_string(),
            )
        })
    }
}

impl TryFrom<&str> for InputData {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self> {
        serde_json::from_str(value).map_err(|e| {
            Error::new_internal(
                String::from("InputData::from_str"),
                String::from("failed to deserialize"),
                e.to_string(),
            )
        })
    }
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
    pub process: Process,         // definition of handling data.
    pub status: State,
    pub input: Option<TaskInput>,

    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    // Function that returns data to be written.
    // Can be None because task can get data straight from connector event.
    pub data_fn: Option<Arc<BoxFnThatReturnsAFuture>>,
    pub callbacks: Option<Vec<CallbackFn>>,

    #[derivative(PartialEq = "ignore")]
    pub kind: Option<TaskKind>, // None if not initialized.
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
            data_fn: data_fn.map(Arc::new),
            spreadsheet_id,
            sheet,
            starting_position,
            direction,
            callbacks: None,
            with_timestamp: false,
            timestamp_position: TimestampPosition::None,
            invocations: None,
            process: Process::default(),
            status: State::Created,
            input: None,
            kind: None,
            kind_request,
        }
    }

    /// Sets TaskKind for TrackingTask, create channels and spawns needed tokio::task for needed types.
    /// Method returns Option<JoinHandle> in order to allow graceful shutdown in scope where this method was called.
    pub async fn init_channels(
        &mut self,
        channels_manager: &ChannelsManager,
        mut shutdown: broadcast::Receiver<()>,
    ) -> Option<JoinHandle<()>> {
        let mut join = None;
        self.kind = Some(match self.kind_request.clone() {
            TaskKindRequest::Triggered(hook) => match hook {
                Hook::None => {
                    let (sender, receiver) = create_channel(1);
                    channels_manager.add_triggered(self.id, sender).await;
                    TaskKind::Triggered {
                        ch: Arc::new(Mutex::new(receiver)),
                    }
                }
                Hook::PSQL {
                    host,
                    port,
                    user,
                    password,
                    db,
                    channel,
                } => {
                    let (sender, receiver) = create_channel(1);
                    join = Some(tokio::task::spawn(async move {
                        monitor_changes(
                            PSQLConfig::new(host, port, user, password, db, Some(channel)),
                            sender,
                            shutdown,
                        )
                        .await;
                    }));
                    TaskKind::Triggered {
                        ch: Arc::new(Mutex::new(receiver)),
                    }
                }
                Hook::Kafka {
                    topic,
                    group_id,
                    brokers,
                } => {
                    let (sender, receiver) = create_channel(1);
                    join = Some(tokio::task::spawn(async move {
                        consume_topic(
                            KafkaConfig {
                                topic,
                                group_id,
                                brokers,
                            },
                            sender,
                            &mut shutdown,
                        )
                        .await;
                    }));
                    TaskKind::Triggered {
                        ch: Arc::new(Mutex::new(receiver)),
                    }
                }
            },
            TaskKindRequest::Clicked => {
                let (sender, receiver) = create_channel::<()>(1);
                channels_manager.add_clicked(self.id, sender).await;
                TaskKind::Clicked {
                    ch: Arc::new(Mutex::new(receiver)),
                }
            }
            TaskKindRequest::Ticker { interval_secs } => TaskKind::Ticker {
                interval: Duration::from_secs(interval_secs),
            },
        });
        join
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

    /// Sets process field.
    pub fn with_process(mut self, process: Process) -> Self {
        self.process = process;
        self
    }

    //TODO: refactor - input and data_fn should be highly connected.
    /// sets input field.
    pub fn with_input(mut self, input: TaskInput) -> TrackingTask {
        self.data_fn = getter_from_task_input(&input).map(Arc::new);
        self.input = Some(input);
        self
    }

    pub fn with_kind(mut self, kind: TaskKind) -> TrackingTask {
        self.kind = Some(kind);
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
        (self.data_fn.as_ref().ok_or_else(|| {
            Error::new_internal(
                String::from("TrackingTask:data"),
                String::from("data_fn is None"),
                String::default(),
            )
        })?)()
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
            process: Process::from(tcr.process),
            data_fn: tcr
                .input
                .as_ref()
                .and_then(|i| getter_from_task_input(i).map(Arc::new)),
            status: State::Created,
            input: tcr.input,
            kind: None,
            kind_request: tcr.kind_request,
        })
    }
}

impl TryFrom<TaskModel> for TrackingTask {
    type Error = Error;

    fn try_from(task_model: TaskModel) -> Result<Self> {
        let id = Uuid::parse_str(&task_model.uuid).map_err(|err| {
            Error::new_internal(
                String::from("from_task_model"),
                String::from("failed to parse uuid"),
                err.to_string(),
            )
        })?;

        let input = task_model
            .input
            .as_ref()
            .map(|i| TaskInput::from_json(i).unwrap());
        let kind_request = TaskKindRequest::from_json(&task_model.kind)?;

        Ok(TrackingTask {
            id,
            name: Some(task_model.name),
            description: Some(task_model.description),
            data_fn: input
                .as_ref()
                .and_then(|i| getter_from_task_input(i).map(Arc::new)),
            spreadsheet_id: task_model.spreadsheet_id,
            starting_position: task_model.position,
            sheet: task_model.sheet,
            direction: task_model.direction,
            with_timestamp: true,
            timestamp_position: task_model.timestamp_position,
            invocations: None,
            process: Process::try_from(task_model.process)?,
            status: task_model.status,
            callbacks: None,
            input,
            kind: None,
            kind_request,
        })
    }
}

mod test {
    #![allow(unused_imports)]
    use crate::core::task::{Direction, InputData, TaskInput, TrackingTask};
    use crate::error::types::Result;
    use crate::server::task::TaskKindRequest;

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
