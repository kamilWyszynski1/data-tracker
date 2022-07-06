use crate::error::types::{Error, Result};
use crate::{core::handler::Report, persistance::interface::Db};
use rocket::http::{ContentType, Status};
use rocket::response::{self, Responder, Response};
use rocket::Request;
use rocket::State;
use serde::Serialize;
use std::str::FromStr;

#[derive(Serialize)]
pub struct Reports {
    reports: Vec<Report>,
}

#[rocket::async_trait]
impl<'r> Responder<'r, 'static> for Reports {
    fn respond_to(self, req: &'r Request<'_>) -> response::Result<'static> {
        let value = serde_json::json!(self);
        Response::build_from(value.respond_to(req).unwrap())
            .status(Status::Ok)
            .header(ContentType::JSON)
            .ok()
    }
}

#[get("/reports/<task_id>")]
pub async fn get_reports(db: &State<Db>, task_id: String) -> Result<Option<Reports>> {
    let uuid = uuid::Uuid::from_str(&task_id).map_err(|e| {
        Error::new_internal(
            String::from("get_reports"),
            String::from("failed to parse uuid"),
            e.to_string(),
        )
    })?;
    Ok(db
        .read_reports(uuid)
        .await?
        .map(|models| -> Vec<Report> { models.into_iter().map(Report::from_model).collect() })
        .map(|reports| {
            debug!("reports: {:?}", reports);
            reports
        })
        .map(|reports| Reports { reports }))
}
