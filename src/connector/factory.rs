use super::*;
use crate::core::task::{BoxFnThatReturnsAFuture, InputData, TaskInput};
use crate::error::types::Result;

fn empty_getter() -> BoxFnThatReturnsAFuture {
    async fn empty() -> Result<InputData> {
        Ok(InputData::String(String::from("")))
    }
    Box::new(move || Box::pin(empty()))
}

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
        TaskInput::None => empty_getter(),
    }
}
