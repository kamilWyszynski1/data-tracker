extern crate datatracker_rust;
use datatracker_rust::core::manager::TaskCommand;
use datatracker_rust::core::task::TrackingTask;
use datatracker_rust::core::tracker::Tracker;
use datatracker_rust::persistance::in_memory::InMemoryPersistance;
use datatracker_rust::web::build::rocket;
use datatracker_rust::wrap::APIWrapper;
use tokio::join;
use tokio::sync::broadcast;
use tokio::sync::mpsc::channel;

#[macro_use]
extern crate log;

#[tokio::main]
async fn main() {
    env_logger::init();
    let (shutdown_notify, shutdown_recv) = broadcast::channel(1);
    let (tt_send, receive) = channel::<TrackingTask>(10);
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

    let rocket = rocket(cmd_send, tt_send);
    let (_, _) = join!(rocket.launch(), start);
}
