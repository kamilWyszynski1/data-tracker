use super::apply::apply;
use super::report::get_reports;
use super::task::create;
use crate::{
    core::{manager::TaskCommand, task::TrackingTask},
    persistance::interface::Db,
};
use rocket::{routes, Build, Rocket};
use tokio::sync::mpsc::Sender;

pub fn rocket(
    cmd_send: Sender<TaskCommand>,
    tt_send: Sender<TrackingTask>,
    db: Db,
) -> Rocket<Build> {
    rocket::build()
        .mount("/", routes![apply, create, get_reports])
        .manage(cmd_send)
        .manage(tt_send)
        .manage(db)
}
