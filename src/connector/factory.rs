use crate::core::task::{BoxFnThatReturnsAFuture, TaskInput};

use super::*;

pub fn getter_from_task_input(input: &TaskInput) -> BoxFnThatReturnsAFuture {
    match input.clone() {
        TaskInput::String { value } => string::getter_from_string(value),
        TaskInput::HTTP { url, input_type } => http::getter_from_url(&url, input_type),
        TaskInput::PSQL {
            host,
            user,
            password,
            query,
        } => psql::getter_from_psql(psql::PSQLConfig::new(host, user, password), query),
        TaskInput::None => todo!(),
    }
}
