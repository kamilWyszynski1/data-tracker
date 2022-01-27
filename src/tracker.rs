use crate::handler::TaskHandler;
use crate::persistance::{Db, Persistance};
use crate::shutdown::Shutdown;
use crate::wrap::API;
use std::marker::{Send, Sync};
use std::sync::Arc;
use std::time::Duration;
use std::vec::Vec;
use tokio::sync::broadcast;
use tokio::sync::mpsc::Receiver;
use uuid::Uuid;

// TrackedData is a type wrap for data that is being tracked. It'll be written as string anyway.
pub type TrackedData = Vec<String>;

// GetDataFn is a type wrap for a function that returns a TrackedData.
type GetDataFn = fn() -> Result<TrackedData, String>;

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
type CallbackFn = fn(Result<(), String>) -> ();

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

    callbacks: Option<Vec<CallbackFn>>,
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
    pub fn run_callbacks(&self, result: Result<(), String>) {
        info!("running callbacks: {:?}", self.callbacks);
        if let Some(callbacks) = &self.callbacks {
            for callback in callbacks {
                callback(result.clone());
            }
        }
    }

    pub fn get_data(&self) -> Result<TrackedData, String> {
        (self.get_data_fn)()
    }

    pub fn get_name(&self) -> &str {
        if let Some(name) = &self.name {
            name
        } else {
            "No name"
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
}

// Tracker is a wrapper for the Google Sheets API.
// It is used to track various kind of things and keep that data in a Google Sheet.
pub struct Tracker<A, P>
where
    A: 'static + API + Sync + Send + Clone,
    P: 'static + Persistance + Send + Clone,
{
    /// Performs write of a data.
    api: A,
    /// Saves last state of handled task.
    persistance: P,
    /// Listen for incoming TrackingTask to handle.
    task_channel: Receiver<TrackingTask>,
    /// Listen for shutdown notifications.
    ///
    /// A wrapper around the `broadcast::Receiver` paired with the sender in
    /// `Listener`. The connection handler processes requests from the
    /// connection until the peer disconnects **or** a shutdown notification is
    /// received from `shutdown`. In the latter case, any in-flight work being
    /// processed for the peer is continued until it reaches a safe state, at
    /// which point the connection is terminated.
    shutdown: Shutdown,
    /// Broadcasts a shutdown signal to all active task handlers..
    notify_shutdown: broadcast::Sender<()>,
}

impl<A, P> Tracker<A, P>
where
    A: 'static + API + Sync + Send + Clone,
    P: 'static + Persistance + Send + Clone,
{
    // creates new Tracker.
    pub fn new(
        api: A,
        persistance: P,
        task_channel: Receiver<TrackingTask>,
        shutdown_channel: broadcast::Receiver<()>,
        notify_shutdown: broadcast::Sender<()>,
    ) -> Self {
        Tracker {
            api,
            task_channel,
            persistance,
            shutdown: Shutdown::new(shutdown_channel),
            notify_shutdown,
        }
    }

    pub async fn start(&mut self) {
        info!("Starting Tracker.");
        let mut spawned = vec![];

        while !self.shutdown.is_shutdown() {
            info!("waiting");
            tokio::select! {
                _ = self.shutdown.recv() => {
                    info!("tracker is shutting down");
                    // If a shutdown signal is received, return from `start`.
                    // This will result in the task terminating.
                    break;
                }
                Some(task) = self.task_channel.recv() => {
                    let mut handler = TaskHandler::new(task, Db::new(self.persistance.clone()), Shutdown::new(self.notify_shutdown.subscribe()), Arc::new(self.api.clone()));
                    spawned.push(tokio::task::spawn(async move {handler.run().await}));
                }
            }
        }
        for (i, s) in spawned.into_iter().enumerate() {
            info!("awaiting {} spawned", i);
            s.await.unwrap();
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
            "spreadsheet_id".to_string(),
            "".to_string(),
            "A1:B1".to_string(),
            Direction::Vertical,
            test_get_data_fn,
            std::time::Duration::from_secs(1),
        );
        tt = tt.with_callback(|res: Result<(), String>| {
            assert!(res.is_ok());
        });
        assert!(tt.callbacks.is_some());
        tt.run_callbacks(Ok(()));
    }

    #[test]
    fn task_with_name() {
        use crate::tracker::{Direction, TrackingTask};
        let mut tt = TrackingTask::new(
            "spreadsheet_id".to_string(),
            "".to_string(),
            "A1:B1".to_string(),
            Direction::Vertical,
            test_get_data_fn,
            std::time::Duration::from_secs(1),
        );
        tt = tt.with_name("test".to_string());
        assert!(tt.name.is_some());
        assert_eq!(tt.name.unwrap(), "test")
    }

    #[test]
    fn task_with_description() {
        use crate::tracker::{Direction, TrackingTask};
        let mut tt = TrackingTask::new(
            "spreadsheet_id".to_string(),
            "".to_string(),
            "A1:B1".to_string(),
            Direction::Vertical,
            test_get_data_fn,
            std::time::Duration::from_secs(1),
        );
        tt = tt.with_description("test".to_string());
        assert!(tt.description.is_some());
        assert_eq!(tt.description.unwrap(), "test")
    }

    use crate::wrap::API;
    use async_trait::async_trait; // crate for async traits.

    #[derive(Clone)]
    struct TestAPI {
        check: fn(Vec<Vec<String>>, &str, &str),
        fail: bool,
        fail_msg: String,
    }

    #[async_trait]
    impl API for TestAPI {
        async fn write(&self, v: Vec<Vec<String>>, s: &str, r: &str) -> Result<(), String> {
            (self.check)(v, s, r);
            if self.fail {
                return Err(self.fail_msg.clone());
            }
            Ok(())
        }
    }
    #[derive(Clone)]
    struct MockAPI {}

    #[async_trait]
    impl API for MockAPI {
        async fn write(&self, _: Vec<Vec<String>>, _: &str, _: &str) -> Result<(), String> {
            Ok(())
        }
    }

    use crate::persistance::Persistance;
    use tokio::sync::broadcast;
    use tokio::sync::mpsc::channel;
    use uuid::Uuid;

    #[derive(Clone)]
    struct TestPersistance {}
    impl Persistance for TestPersistance {
        fn write(&mut self, _: Uuid, _: u32) -> Result<(), String> {
            Ok(())
        }
        fn read(&self, _: &Uuid) -> Option<u32> {
            None
        }
    }

    #[tokio::test]
    #[timeout(10000)]
    async fn test_send_receive() {
        use tokio::sync::oneshot;
        let (tx, rx) = oneshot::channel::<bool>();

        fn check_cases(v: Vec<Vec<String>>, s: &str, r: &str) {
            let cases = vec![
                (vec![vec!["test".to_string()]], "spreadsheet4", "A4:A5"),
                (vec![vec!["test".to_string()]], "spreadsheet5", "A5:A6"),
            ];
            info!("{:?} {} {}", cases, s, r);
            for (i, c) in cases.iter().enumerate() {
                if v == c.0 && s == c.1 && r == c.2 {
                    info!("Case {} passed", i);
                    return;
                }
            }
            panic!("failed")
        }

        fn callback(_: Result<(), String>) {}

        let c = |tx: oneshot::Sender<bool>| -> fn(Result<(), String>) {
            info!("callback");
            tx.send(true).unwrap();
            callback
        };

        let (shutdown_notify, shutdown) = broadcast::channel(1);
        let (send, receive) = channel::<TrackingTask>(1);

        use crate::tracker::{Direction, Tracker, TrackingTask};
        let mut t = Tracker::new(
            TestAPI {
                check: check_cases,
                fail: false,
                fail_msg: "".to_string(),
            },
            TestPersistance {},
            receive,
            shutdown,
            shutdown_notify,
        );
        tokio::task::spawn(async move {
            t.start().await;
        });
        info!("started");
        assert!(send
            .send(
                TrackingTask::new(
                    "spreadsheet4".to_string(),
                    "".to_string(),
                    "A4".to_string(),
                    Direction::Vertical,
                    test_get_data_fn,
                    std::time::Duration::from_secs(1),
                )
                .with_name("TEST4".to_string())
                .with_invocations(1),
            )
            .await
            .is_ok());
        assert!(send
            .send(
                TrackingTask::new(
                    "spreadsheet5".to_string(),
                    "".to_string(),
                    "A5".to_string(),
                    Direction::Vertical,
                    test_get_data_fn,
                    std::time::Duration::from_secs(1),
                )
                .with_name("TEST5".to_string())
                .with_callback(c(tx))
                .with_invocations(1),
            )
            .await
            .is_ok());
        rx.await.unwrap();
    }
}
