use crate::core::task::{BoxFnThatReturnsAFuture, InputData};
use crate::core::types::InputType;
use crate::error::types::{Error, Result};

pub fn getter_from_url(url: &str, it: InputType) -> BoxFnThatReturnsAFuture {
    let u = url.to_string();
    Box::new(move || Box::pin(get(u.clone(), it)))
}

async fn get(url: String, it: InputType) -> Result<InputData> {
    match reqwest::get(url).await {
        Ok(r) => match it {
            InputType::String => r
                .text()
                .await
                .map_err(|err| {
                    Error::new_internal(
                        String::from("get"),
                        String::from("failed to get text"),
                        err.to_string(),
                    )
                })
                .map(InputData::String),
            InputType::Json => r
                .json()
                .await
                .map_err(|err| {
                    Error::new_internal(
                        String::from("get"),
                        String::from("failed to get json"),
                        err.to_string(),
                    )
                })
                .map(InputData::Json),
        },
        Err(err) => {
            error!("failed to get data {}", err);
            Err(Error::new_internal(
                String::from("get"),
                String::from("failed to get data from url"),
                err.to_string(),
            ))
        }
    }
}
