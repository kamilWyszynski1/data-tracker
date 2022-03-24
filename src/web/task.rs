use crate::lang::engine::Definition;
use crate::tracker::task::TrackingTask;
use rocket::http::{ContentType, Status};
use rocket::response::{self, Responder, Response};
use rocket::serde::json::Json;
use rocket::{Request, State};
use serde::Deserialize;
use serde_json::json;
use serde_json::Value;
use tokio::sync::mpsc::Sender;

#[derive(Deserialize, Clone)]
pub struct TaskCreateRequest {
    pub name: String,
    pub description: String,
    pub spreadsheet_id: String,
    pub sheet: String,
    pub starting_position: String,
    pub direction: String,
    pub interval_secs: u64,
    pub definition: Definition,
    pub url: String, // url for acquiring the data.
}

pub struct TaskCreateResponse {
    pub json: Value,
    pub status: Status,
}

impl TaskCreateResponse {
    fn new(json: Value, status: Status) -> Self {
        TaskCreateResponse { json, status }
    }
}

#[rocket::async_trait]
impl<'r> Responder<'r, 'static> for TaskCreateResponse {
    fn respond_to(self, req: &'r Request<'_>) -> response::Result<'static> {
        Response::build_from(self.json.respond_to(req).unwrap())
            .status(self.status)
            .header(ContentType::JSON)
            .ok()
    }
}

pub async fn apply(
    sender: &State<Sender<TrackingTask>>,
    request: Json<TaskCreateRequest>,
) -> TaskCreateResponse {
    let tt = TrackingTask::from_task_create_request(request.clone());

    match tt {
        Ok(tt) => {
            let id = tt.id().clone();
            match sender.send(tt).await {
                Ok(_) => TaskCreateResponse::new(json!({ "id": id }), Status::Ok),
                Err(e) => {
                    error!("{}", e);
                    TaskCreateResponse::new(json!({ "err": format!("{}", e) }), Status::Ok)
                }
            }
        }
        Err(e) => {
            error!("{}", e);
            TaskCreateResponse::new(json!({ "err": format!("{}", e) }), Status::Ok)
        }
    }
}
