extern crate datatracker_rust;
use datatracker_rust::persistance::in_memory::InMemoryPersistance;
use datatracker_rust::tracker::task::{random_value_generator, Direction, TrackingTask};
use datatracker_rust::tracker::tracker::Tracker;
use datatracker_rust::web::server::{router, GenericError, Result};
use datatracker_rust::wrap::APIWrapper;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Client, Server};
use std::time::Duration;
use tokio::join;
use tokio::sync::broadcast;
use tokio::sync::mpsc::channel;

#[macro_use]
extern crate log;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    let (shutdown_notify, shutdown_recv) = broadcast::channel(1);
    let (send, receive) = channel::<TrackingTask>(10);

    let api = APIWrapper::new_with_init().await;
    let mem = InMemoryPersistance::new();
    let mut tracker = Tracker::new(api, mem, receive, shutdown_recv, shutdown_notify.clone());
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

    // create web server.
    let addr = "127.0.0.1:1337".parse().unwrap();

    // Share a `Client` with all `Service`s
    let client = Client::new();

    let new_service = make_service_fn(move |_| {
        // Move a clone of `client` into the `service_fn`.
        let client = client.clone();
        async {
            Ok::<_, GenericError>(service_fn(move |req| {
                // Clone again to ensure that client outlives this closure.
                router(req, client.to_owned())
            }))
        }
    });

    let server = Server::bind(&addr)
        .serve(new_service)
        .with_graceful_shutdown(shutdown_signal());

    println!("Listening on http://{}", addr);

    join!(start, server);
    // server.await?;
    Ok(())
}

async fn shutdown_signal() {
    // Wait for the CTRL+C signal
    tokio::signal::ctrl_c()
        .await
        .expect("failed to install CTRL+C signal handler");
}
