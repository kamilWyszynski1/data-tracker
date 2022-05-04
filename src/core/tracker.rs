use super::handler::TaskHandler;
use super::manager::{SenderManager, TaskCommand};
use super::task::TrackingTask;
use super::types::State;
use crate::error::types::Result;
use crate::persistance::interface::Db;
use crate::shutdown::Shutdown;
use crate::wrap::API;
use std::marker::{Send, Sync};
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::sync::mpsc::Receiver;
use tokio::task::JoinHandle;

// Tracker is a wrapper for the Google Sheets API.
// It is used to track various kind of things and keep that data in a Google Sheet.
pub struct Tracker<A>
where
    A: 'static + API + Sync + Send,
{
    /// Performs write of a data.
    api: Arc<A>,
    /// Saves last state of handled task.
    db: Db,
    /// Listen for incoming TrackingTask to handle.
    task_channel: Receiver<TrackingTask>,
    /// Listen for incoming Command for Task to handle.
    task_command_channel: Receiver<TaskCommand>,

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
    manager: SenderManager,
}

impl<A> Tracker<A>
where
    A: 'static + API + Sync + Send,
{
    // creates new Tracker.
    pub fn new(
        api: A,
        db: Db,
        task_channel: Receiver<TrackingTask>,
        shutdown_channel: broadcast::Receiver<()>,
        notify_shutdown: broadcast::Sender<()>,
        task_command_channel: Receiver<TaskCommand>,
    ) -> Self {
        Tracker {
            api: Arc::new(api),
            task_channel,
            task_command_channel,
            db,
            shutdown: Shutdown::new(shutdown_channel),
            notify_shutdown,
            manager: SenderManager::default(),
        }
    }

    pub async fn start(&mut self) {
        info!("Starting Tracker.");
        let mut spawned = vec![];
        if let Err(e) = self.load_from_db(&mut spawned).await {
            error!("{:?}", e);
        }

        while !self.shutdown.is_shutdown() {
            tokio::select! {
                _ = self.shutdown.recv() => {
                    info!("tracker is shutting down");
                    // If a shutdown signal is received, return from `start`.
                    // This will result in the task terminating.
                    break;
                }
                Some(task) = self.task_channel.recv() => {
                    if let Err(e) = self.receive_task(&task, &mut spawned).await {
                        error!("{:?}", e);
                    }
                }
                Some(task_cmd) = self.task_command_channel.recv() => {
                    self.manager.apply(task_cmd.id, task_cmd.cmd).await;
                }
            }
        }

        for (i, s) in spawned.into_iter().enumerate() {
            info!("awaiting {} spawned", i);
            s.await.unwrap();
        }
    }

    /// Gets task, creates new TaskHandler and pushes it to spawned handlers.
    /// TaskHandler will start its work waiting for shutdown message.
    async fn receive_task(
        &mut self,
        task: &TrackingTask,
        spawned: &mut Vec<JoinHandle<()>>,
    ) -> Result<()> {
        if task.status == State::Created {
            debug!("saving task on receive: {:?}", task);
            self.db.save_task(task).await?;
        }
        self.start_handler_for_task(task, spawned).await;
        Ok(())
    }

    /// Creates new TaskHandler for given task and pushes it to vector of handlers.
    async fn start_handler_for_task(
        &mut self,
        task: &TrackingTask,
        spawned: &mut Vec<JoinHandle<()>>,
    ) {
        info!(
            "start_handler_for_task - for {}:{} task",
            task.id, task.status
        );
        let mut handler = TaskHandler::new_ticker(
            task.clone(),
            self.db.clone(),
            Shutdown::new(self.notify_shutdown.subscribe()),
            self.api.clone(),
            self.manager.add_new_mapping(task.id),
        );
        spawned.push(tokio::task::spawn(async move { handler.start().await }));
    }

    /// Load saved task from DB and start handling them.
    async fn load_from_db(&mut self, spawned: &mut Vec<JoinHandle<()>>) -> Result<()> {
        let tasks = self
            .db
            .get_tasks_by_status(&[State::Running, State::Stopped, State::Created])
            .await?;
        info!("{} tasks loaded from db", tasks.len());

        for tt in &tasks {
            debug!("task from database: {:?}", tt);
            self.start_handler_for_task(tt, spawned).await;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::core::task::*;
    use crate::core::tracker::Tracker;
    use crate::error::types::{Error, Result};
    use crate::wrap::{MockAPI, API};
    use async_trait::async_trait; // crate for async traits.

    async fn test_get_data_fn() -> Result<InputData> {
        Ok(InputData::String(String::from("test")))
    }

    #[derive(Clone)]
    struct TestAPI {
        check: fn(Vec<Vec<String>>, &str, &str),
        fail: bool,
        fail_msg: &'static str,
    }

    #[async_trait]
    impl API for TestAPI {
        async fn write(&self, v: Vec<Vec<String>>, s: &str, r: &str) -> Result<()> {
            (self.check)(v, s, r);
            if self.fail {
                return Err(Error::new_internal(
                    String::from("write"),
                    String::from("mock error"),
                    self.fail_msg.to_string(),
                ));
            }
            Ok(())
        }
    }

    use crate::core::tracker::TaskCommand;
    use crate::core::types::*;
    use crate::persistance::interface::{Db, MockPersistance};
    use tokio::sync::broadcast;
    use tokio::sync::mpsc::channel;

    #[tokio::test]
    #[timeout(10000)]
    async fn test_send_receive() {
        use mockall::predicate::*;
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

        fn callback(_: Result<()>) {}

        let c = |tx: oneshot::Sender<bool>| -> fn(Result<()>) {
            info!("callback");
            tx.send(true).unwrap();
            callback
        };

        let (shutdown_notify, shutdown) = broadcast::channel(1);
        let (send, receive) = channel::<TrackingTask>(1);
        let (_cmd_send, cmd_receive) = channel::<TaskCommand>(1);

        let fail_msg = "";

        let t1 = TrackingTask::new(
            "spreadsheet4".to_string(),
            "".to_string(),
            "A4".to_string(),
            Direction::Vertical,
            Box::new(move || Box::pin(test_get_data_fn())),
            std::time::Duration::from_secs(1),
        )
        .with_name("TEST4".to_string())
        .with_invocations(1);
        let t2 = TrackingTask::new(
            "spreadsheet5".to_string(),
            "".to_string(),
            "A5".to_string(),
            Direction::Vertical,
            Box::new(move || Box::pin(test_get_data_fn())),
            std::time::Duration::from_secs(1),
        )
        .with_name("TEST5".to_string())
        .with_callback(c(tx))
        .with_invocations(1);

        let mut mock_persistence = MockPersistance::new();
        mock_persistence
            .expect_save_task()
            .with(eq(t1.clone()))
            .once()
            .returning(|_| Ok(()));
        mock_persistence
            .expect_save_task()
            .with(eq(t2.clone()))
            .once()
            .returning(|_| Ok(()));
        mock_persistence
            .expect_get_tasks_by_status()
            .withf(|x: &[State]| x == &[State::Running, State::Stopped, State::Created])
            .once()
            .returning(|_| Ok(vec![]));

        let mut t = Tracker::new(
            TestAPI {
                check: check_cases,
                fail: false,
                fail_msg: fail_msg,
            },
            Db::new(Box::new(mock_persistence)),
            receive,
            shutdown,
            shutdown_notify,
            cmd_receive,
        );
        tokio::task::spawn(async move {
            t.start().await;
        });
        info!("started");
        assert!(send.send(t1).await.is_ok());
        assert!(send.send(t2).await.is_ok());
        rx.await.unwrap();
    }

    #[tokio::test]
    #[timeout(10000)]
    async fn test_saved_tasks() {
        use mockall::predicate::*;
        use tokio::sync::oneshot;
        let (tx, rx) = oneshot::channel::<bool>();

        let mut t1 = TrackingTask::new(
            "spreadsheet4".to_string(),
            "".to_string(),
            "A4".to_string(),
            Direction::Vertical,
            Box::new(move || Box::pin(test_get_data_fn())),
            std::time::Duration::from_secs(1),
        )
        .with_name("TEST4".to_string());
        t1.status = State::Running;

        let mut t2 = TrackingTask::new(
            "spreadsheet5".to_string(),
            "".to_string(),
            "A5".to_string(),
            Direction::Vertical,
            Box::new(move || Box::pin(test_get_data_fn())),
            std::time::Duration::from_secs(1),
        )
        .with_name("TEST5".to_string())
        .with_invocations(1);
        t2.status = State::Stopped;

        let mut mock_persistence = MockPersistance::new();
        mock_persistence
            .expect_get_tasks_by_status()
            .withf(|x: &[State]| x == &[State::Running, State::Stopped, State::Created])
            .once()
            .returning(move |_| Ok(vec![t1.clone(), t2.clone()]));

        let (shutdown_notify, shutdown) = broadcast::channel(1);
        let (send, receive) = channel::<TrackingTask>(1);
        let (_cmd_send, cmd_receive) = channel::<TaskCommand>(1);

        let mut t = Tracker::new(
            MockAPI::new(),
            Db::new(Box::new(mock_persistence)),
            receive,
            shutdown,
            shutdown_notify,
            cmd_receive,
        );
        tokio::task::spawn(async move {
            t.start().await;
        });
    }
}
