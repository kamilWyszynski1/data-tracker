use crate::core::task::{BoxFnThatReturnsAFuture, InputData};
use crate::error::types::Result;

pub fn getter_from_string(s: String) -> BoxFnThatReturnsAFuture {
    Box::new(move || Box::pin(return_string(s.clone())))
}

async fn return_string(s: String) -> Result<InputData> {
    Ok(InputData::String(s))
}
