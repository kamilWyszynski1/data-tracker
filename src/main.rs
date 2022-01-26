extern crate datatracker_rust;

use datatracker_rust::persistance::InMemoryPersistance;
use datatracker_rust::task::random_value_generator;
use datatracker_rust::tracker::{Direction, Tracker, TrackingTask};
use datatracker_rust::wrap::APIWrapper;
use std::time::Duration;

#[tokio::main]
async fn main() {
    let api = APIWrapper::new_with_init().await;
    let mem = InMemoryPersistance::new();
    let mut tracker = Tracker::new(api, mem);
    tracker.start().await;
    let task = TrackingTask::new(
        "12rVPMk3Lv7VouUZBglDd_oRDf6PHU7m6YbfctmFYYlg".to_string(),
        "".to_string(),
        "A1".to_string(),
        Direction::Horizontal,
        random_value_generator,
        Duration::from_secs(10),
    )
    .with_name("Writing random stuff to sheets".to_string())
    .with_callback(|r: Result<(), String>| println!("{:?}", r));
    tracker.send_task(task).await;
    tokio::time::sleep(Duration::from_secs(60)).await;
}
