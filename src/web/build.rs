use super::router::{apply, index};
use crate::tracker::manager::TaskCommand;
use rocket::{Build, Rocket};
use tokio::sync::mpsc::Sender;

pub fn rocket(cmd_send: Sender<TaskCommand>) -> Rocket<Build> {
    rocket::build()
        .mount("/", routes![index, apply])
        .manage(cmd_send)
}
