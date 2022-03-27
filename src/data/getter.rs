use crate::tracker::task::{BoxFnThatReturnsAFuture, InputData, InputType};

pub fn getter_from_url(url: &str, it: InputType) -> BoxFnThatReturnsAFuture {
    let u = url.to_string();
    Box::new(move || Box::pin(get(u.clone(), it)))
}

async fn get(url: String, it: InputType) -> Result<InputData, &'static str> {
    match reqwest::get(url).await {
        Ok(r) => match it {
            InputType::String => r
                .text()
                .await
                .map_err(|err| {
                    error!("failed to get text {}", err);
                    "failed to get text"
                })
                .and_then(|t| Ok(InputData::String(t))),
            InputType::Json => r
                .json()
                .await
                .map_err(|err| {
                    error!("failed to get json {}", err);
                    "failed to get json"
                })
                .and_then(|t| Ok(InputData::Json(t))),
        },
        Err(e) => {
            error!("failed to get data {}", e);
            Err("failed to get data")
        }
    }
}
