use super::handler::TaskHandler;
use super::manager::{SenderManager, TaskCommand};
use super::task::TrackingTask;
use crate::persistance::interface::{Db, Persistance};
use crate::shutdown::Shutdown;
use crate::wrap::API;
use std::marker::{Send, Sync};
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::sync::mpsc::Receiver;

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
        task_command_channel: Receiver<TaskCommand>,
    ) -> Self {
        Tracker {
            api,
            task_channel,
            task_command_channel,
            persistance,
            shutdown: Shutdown::new(shutdown_channel),
            notify_shutdown,
            manager: SenderManager::default(),
        }
    }

    pub async fn start(&mut self) {
        info!("Starting Tracker.");
        let mut spawned = vec![];
        while !self.shutdown.is_shutdown() {
            tokio::select! {
                _ = self.shutdown.recv() => {
                    info!("tracker is shutting down");
                    // If a shutdown signal is received, return from `start`.
                    // This will result in the task terminating.
                    break;
                }
                Some(task) = self.task_channel.recv() => {
                    let receiver = self.manager.add_new_mapping(task.id());
                    let mut handler = TaskHandler::new(task, Db::new(self.persistance.clone()), Shutdown::new(self.notify_shutdown.subscribe()), Arc::new(self.api.clone()), receiver);
                    spawned.push(tokio::task::spawn(async move {handler.start().await}));
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
}

#[cfg(test)]
mod tests {
    use crate::tracker::task::{Direction, TrackedData, TrackingTask};
    use crate::tracker::tracker::Tracker;
    use crate::wrap::API;
    use async_trait::async_trait; // crate for async traits.

    fn test_get_data_fn() -> Result<TrackedData, &'static str> {
        Ok(vec!["test".to_string()])
    }

    #[derive(Clone)]
    struct TestAPI {
        check: fn(Vec<Vec<String>>, &str, &str),
        fail: bool,
        fail_msg: &'static str,
    }

    #[async_trait]
    impl API for TestAPI {
        async fn write(&self, v: Vec<Vec<String>>, s: &str, r: &str) -> Result<(), &'static str> {
            (self.check)(v, s, r);
            if self.fail {
                return Err(self.fail_msg);
            }
            Ok(())
        }
    }
    #[derive(Clone)]
    struct MockAPI {}

    #[async_trait]
    impl API for MockAPI {
        async fn write(&self, _: Vec<Vec<String>>, _: &str, _: &str) -> Result<(), &'static str> {
            Ok(())
        }
    }

    use crate::persistance::interface::Persistance;
    use crate::tracker::tracker::TaskCommand;
    use tokio::sync::broadcast;
    use tokio::sync::mpsc::channel;
    use uuid::Uuid;

    #[derive(Clone)]
    struct TestPersistance {}
    impl Persistance for TestPersistance {
        fn write(&mut self, _: Uuid, _: u32) -> Result<(), &'static str> {
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

        fn callback(_: Result<(), &'static str>) {}

        let c = |tx: oneshot::Sender<bool>| -> fn(Result<(), &'static str>) {
            info!("callback");
            tx.send(true).unwrap();
            callback
        };

        let (shutdown_notify, shutdown) = broadcast::channel(1);
        let (send, receive) = channel::<TrackingTask>(1);
        let (_cmd_send, cmd_receive) = channel::<TaskCommand>(1);

        let fail_msg = "";

        let mut t = Tracker::new(
            TestAPI {
                check: check_cases,
                fail: false,
                fail_msg: fail_msg,
            },
            TestPersistance {},
            receive,
            shutdown,
            shutdown_notify,
            cmd_receive,
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
