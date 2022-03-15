extern crate datatracker_rust;
use datatracker_rust::persistance::in_memory::InMemoryPersistance;
use datatracker_rust::tracker::manager::TaskCommand;
use datatracker_rust::tracker::task::{random_value_generator, Direction, TrackingTask};
use datatracker_rust::tracker::tracker::Tracker;
use datatracker_rust::wrap::APIWrapper;
use std::time::Duration;
use tokio::sync::broadcast;
use tokio::sync::mpsc::channel;

#[macro_use]
extern crate log;

#[tokio::main]
async fn main() {
    env_logger::init();
    let (shutdown_notify, shutdown_recv) = broadcast::channel(1);
    let (send, receive) = channel::<TrackingTask>(10);
    let (cmd_send, cmd_receive) = channel::<TaskCommand>(10);

    let api = APIWrapper::new_with_init().await;
    let mem = InMemoryPersistance::new();
    let mut tracker = Tracker::new(
        api,
        mem,
        receive,
        shutdown_recv,
        shutdown_notify.clone(),
        cmd_receive,
    );
    info!("initialized");

    tokio::task::spawn(async move {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                // The shutdown signal has been received.
                shutdown_notify.send(()).unwrap();
                info!("shutting down");
            }
        }
    });
    let start = tokio::task::spawn(async move {
        tracker.start().await;
    });
    let task = TrackingTask::new(
        "12rVPMk3Lv7VouUZBglDd_oRDf6PHU7m6YbfctmFYYlg".to_string(),
        "".to_string(),
        "A1".to_string(),
        Direction::Vertical,
        random_value_generator,
        Duration::from_secs(10),
    )
    .with_name("TASK_1".to_string())
    .with_callback(|r: std::result::Result<(), String>| info!("callback: {:?}", r));
    assert!(send.send(task).await.is_ok());
}
