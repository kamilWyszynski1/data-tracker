use crate::tracker::task::{BoxFnThatReturnsAFuture, InputData, InputType};

pub fn getter_from_url(url: &str, it: InputType) -> BoxFnThatReturnsAFuture {
    let u = url.to_string();
    Box::new(move || Box::pin(get(u.clone(), it)))
}

async fn get(url: String, it: InputType) -> Result<InputData, &'static str> {
    match reqwest::blocking::get(url) {
        Ok(r) => match it {
            InputType::String => r
                .text()
                .map_err(|err| {
                    error!("failed to get text {}", err);
                    "failed to get text"
                })
                .and_then(|t| Ok(InputData::String(t))),
            InputType::Json => r
                .text()
                .map_err(|err| {
                    error!("failed to get json {}", err);
                    "failed to get json"
                })
                .and_then(|t| Ok(InputData::String(t))),
        },
        Err(e) => {
            error!("failed to get data {}", e);
            Err("failed to get data")
        }
    }
}
