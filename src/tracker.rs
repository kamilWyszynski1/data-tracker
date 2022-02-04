use crate::handler::TaskHandler;
use crate::persistance::{Db, Persistance};
use crate::shutdown::Shutdown;
use crate::task::TrackingTask;
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
    use crate::task::{Direction, TrackedData, TrackingTask};
    use crate::tracker::Tracker;
    use crate::wrap::API;
    use async_trait::async_trait; // crate for async traits.

    fn test_get_data_fn() -> Result<TrackedData, String> {
        Ok(vec!["test".to_string()])
    }

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
