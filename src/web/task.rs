use crate::core::direction::Direction;
use crate::core::intype::InputType;
use crate::core::task::TrackingTask;
use crate::lang::engine::Definition;
use rocket::http::{ContentType, Status};
use rocket::response::{self, Responder, Response};
use rocket::serde::json::Json;
use rocket::{Request, State};
use serde::Deserialize;
use serde_json::json;
use serde_json::Value;
use tokio::sync::mpsc::Sender;

#[derive(Debug, Deserialize, Clone)]
pub struct TaskCreateRequest {
    pub name: String,
    pub description: String,
    pub spreadsheet_id: String,
    pub sheet: String,
    pub starting_position: String,
    pub direction: Direction,
    pub interval_secs: u64,
    pub definition: Definition,
    pub input_type: InputType,
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

#[post("/create", format = "json", data = "<request>")]
pub async fn create(
    sender: &State<Sender<TrackingTask>>,
    request: Json<TaskCreateRequest>,
) -> TaskCreateResponse {
    info!("definition from request: {:?}", request.definition);

    let tt = TrackingTask::from_task_create_request(request.clone());

    match tt {
        Ok(tt) => {
            let id = tt.id;
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

#[cfg(test)]
mod tests {
    use crate::web::task::TaskCreateRequest;

    #[test]
    fn test_deserializing() {
        let req: TaskCreateRequest = serde_json::from_str(
            r#"{
            "name": "name",
            "description": "description",
            "spreadsheet_id": "id",
            "sheet": "sheet",
            "starting_position": "A1",
            "direction": "horizontal",
            "interval_secs": 30,
            "definition": {
                "steps": [
                    "DEFINE(var, VEC(1,2,3,4, GET(IN))",
                    "DEFINE(OUT, GET(var))"
                ]
            },
            "input_type": "json",
            "url": "whatever"
        }"#,
        )
        .unwrap();
        assert_eq!(req.definition.steps.len(), 2);
        println!("{:?}", req);
    }
}
