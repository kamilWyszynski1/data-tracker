use crate::core::task::{TaskInput, TrackingTask};
use crate::core::types::*;
use crate::error::types::{Error, Result};
use crate::lang::engine::Definition;
use rocket::http::{ContentType, Status};
use rocket::response::{self, Responder, Response};
use rocket::serde::json::Json;
use rocket::{Request, State};
use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_json::Value;
use tokio::sync::mpsc::Sender;

#[derive(Debug, Deserialize, Clone, PartialEq, Serialize)]
// Will be translated into [`core::types::TaskKind`];
pub enum TaskKindRequest {
    Ticker { interval_secs: u64 }, // task with ticker.
    Triggered(Hook),
    Clicked, // creation should return url that can trigger action.
}

impl TaskKindRequest {
    pub fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json).map_err(|err| {
            Error::new_internal(
                String::from("from_string"),
                String::from("failed to deserialize task input"),
                err.to_string(),
            )
        })
    }
    pub fn to_json(&self) -> String {
        serde_json::json!(self).to_string()
    }
}

#[derive(Debug, Deserialize, Clone, PartialEq)]
pub struct TaskCreateRequest {
    pub name: String,
    pub description: String,
    pub spreadsheet_id: String,
    pub sheet: String,
    pub starting_position: String,
    pub direction: Direction,
    pub definition: Definition,
    pub input: Option<TaskInput>,
    pub kind_request: TaskKindRequest,
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
    use crate::core::task::TaskInput;
    use crate::core::types::Hook;
    use crate::server::task::TaskKindRequest;
    use crate::{
        core::types::Direction, lang::engine::Definition, server::task::TaskCreateRequest,
    };

    #[test]
    fn proper_test_deserializing() {
        struct TestCase {
            name: &'static str,
            json: &'static str,
            wanted: TaskCreateRequest,
            want_err: bool,
        }

        vec![
            TestCase {
                name: "TaskKind::Ticker",
                json: r#"{
                "name": "name",
                "description": "description",
                "spreadsheet_id": "id",
                "sheet": "sheet",
                "starting_position": "A1",
                "direction": "horizontal",
                "definition": {
                    "steps": [
                        "DEFINE(var, VEC(1,2,3,4, GET(IN))",
                        "DEFINE(OUT, GET(var))"
                    ]
                },
                "input_type": "json",
                "url": "whatever",
                "input": "None",
                "kind_request": {"Ticker": {"interval_secs": 30}}
            }"#,
                wanted: TaskCreateRequest {
                    name: String::from("name"),
                    description: String::from("description"),
                    spreadsheet_id: String::from("id"),
                    sheet: String::from("sheet"),
                    starting_position: String::from("A1"),
                    direction: Direction::Horizontal,
                    definition: Definition::new(vec![
                        String::from("DEFINE(var, VEC(1,2,3,4, GET(IN))"),
                        String::from("DEFINE(OUT, GET(var))"),
                    ]),
                    input: Some(TaskInput::None),
                    kind_request: TaskKindRequest::Ticker{ interval_secs: 30 },
                },
                want_err: false,
            },
            TestCase {
                name: "basic",
                json: r#"{
                "name": "name",
                "description": "description",
                "spreadsheet_id": "id",
                "sheet": "sheet",
                "starting_position": "A1",
                "direction": "horizontal",
                "definition": {
                    "steps": [
                        "DEFINE(var, VEC(1,2,3,4, GET(IN))",
                        "DEFINE(OUT, GET(var))"
                    ]
                },
                "input_type": "json",
                "url": "whatever",
                "input": "None",
                "kind_request": "Clicked"
            }"#,
                wanted: TaskCreateRequest {
                    name: String::from("name"),
                    description: String::from("description"),
                    spreadsheet_id: String::from("id"),
                    sheet: String::from("sheet"),
                    starting_position: String::from("A1"),
                    direction: Direction::Horizontal,
                    definition: Definition::new(vec![
                        String::from("DEFINE(var, VEC(1,2,3,4, GET(IN))"),
                        String::from("DEFINE(OUT, GET(var))"),
                    ]),
                    input: Some(TaskInput::None),
                    kind_request:             TaskKindRequest::Clicked,
                },
                want_err: false,
            },
            TestCase {
                name: "TaskInput::PSQL",
                json: r#"{
                "name": "name",
                "description": "description",
                "spreadsheet_id": "id",
                "sheet": "sheet",
                "starting_position": "A1",
                "direction": "horizontal",
                "definition": {
                    "steps": [
                        "DEFINE(var, VEC(1,2,3,4, GET(IN))",
                        "DEFINE(OUT, GET(var))"
                    ]
                },
                "input_type": "json",
                "url": "whatever",
                "kind_request": "Clicked",
                "input": {"PSQL":{"db":"test","host":"host","password":"pass","port":5432,"query":"SELECT 1","user":"user"}}
            }"#,
                wanted: TaskCreateRequest {
                    name: String::from("name"),
                    description: String::from("description"),
                    spreadsheet_id: String::from("id"),
                    sheet: String::from("sheet"),
                    starting_position: String::from("A1"),
                    direction: Direction::Horizontal,
                    definition: Definition::new(vec![
                        String::from("DEFINE(var, VEC(1,2,3,4, GET(IN))"),
                        String::from("DEFINE(OUT, GET(var))"),
                    ]),
                    input: Some(TaskInput::PSQL {
                        host: String::from("host"),
                        port: 5432,
                        user: String::from("user"),
                        password: String::from("pass"),
                        query: String::from("SELECT 1"),
                        db: String::from("test"),
                    }),
                    kind_request: TaskKindRequest::Clicked,
                },
                want_err: false,
            },
            TestCase {
                name: "TaskKind::Triggered",
                json: r#"{
                "name": "name",
                "description": "description",
                "spreadsheet_id": "id",
                "sheet": "sheet",
                "starting_position": "A1",
                "direction": "horizontal",
                "definition": {
                    "steps": [
                        "DEFINE(var, VEC(1,2,3,4, GET(IN))",
                        "DEFINE(OUT, GET(var))"
                    ]
                },
                "input_type": "json",
                "url": "whatever",
                "kind_request": {"Triggered": {"PSQL": {"db":"test","host":"host","password":"pass","port":5432,"channel":"channel","user":"user"}}}
            }"#,
                wanted: TaskCreateRequest {
                    name: String::from("name"),
                    description: String::from("description"),
                    spreadsheet_id: String::from("id"),
                    sheet: String::from("sheet"),
                    starting_position: String::from("A1"),
                    direction: Direction::Horizontal,
                    definition: Definition::new(vec![
                        String::from("DEFINE(var, VEC(1,2,3,4, GET(IN))"),
                        String::from("DEFINE(OUT, GET(var))"),
                    ]),
                    input: None,
                    kind_request: TaskKindRequest::Triggered(Hook::PSQL{
                        host: String::from("host"),
                        port: 5432,
                        user: String::from("user"),
                        password: String::from("pass"),
                        channel: String::from("channel"),
                        db: String::from("test"),
                    }),
                },
                want_err: false,
            },
        ]
        .into_iter()
        .for_each(|c| {
            let req: TaskCreateRequest = serde_json::from_str(c.json).unwrap();
            assert_eq!(
                req, c.wanted,
            );
        });
    }
}
