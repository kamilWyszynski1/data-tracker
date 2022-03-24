use crate::tracker::task::GetDataFn;

pub fn getter_from_url(url: &str) -> GetDataFn {
    || Ok(vec![String::from("1")])
}
