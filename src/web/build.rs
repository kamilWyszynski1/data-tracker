use super::apply::apply;
use super::task::create;
use crate::tracker::{manager::TaskCommand, task::TrackingTask};
use rocket::{routes, Build, Rocket};
use tokio::sync::mpsc::Sender;

pub fn rocket(cmd_send: Sender<TaskCommand>, tt_send: Sender<TrackingTask>) -> Rocket<Build> {
    rocket::build()
        .mount("/", routes![apply, create])
        .manage(cmd_send)
        .manage(tt_send)
}
