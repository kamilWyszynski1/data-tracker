use super::apply::apply;
use crate::tracker::manager::TaskCommand;
use rocket::{routes, Build, Rocket};
use tokio::sync::mpsc::Sender;

pub fn rocket(cmd_send: Sender<TaskCommand>) -> Rocket<Build> {
    rocket::build().mount("/", routes![apply]).manage(cmd_send)
}
