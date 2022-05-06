use super::*;
use crate::core::task::{BoxFnThatReturnsAFuture, TaskInput};

pub fn getter_from_task_input(input: &TaskInput) -> Option<BoxFnThatReturnsAFuture> {
    match input.clone() {
        TaskInput::String { value } => Some(string::getter_from_string(value)),
        TaskInput::HTTP { url, input_type } => Some(http::getter_from_url(&url, input_type)),
        TaskInput::PSQL {
            host,
            port,
            user,
            password,
            query,
            db,
        } => Some(psql::getter_from_psql(
            psql::PSQLConfig::new(host, port, user, password, db, None),
            query,
        )),
        TaskInput::None => None,
    }
}
