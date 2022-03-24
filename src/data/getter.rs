use crate::tracker::task::GetDataFn;

pub fn getter_from_url(_: &str) -> GetDataFn {
    || Ok(vec![String::from("1")])
}
