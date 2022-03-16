use crate::tracker::manager::{Command, TaskCommand};
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use serde::Deserialize;
use tokio::sync::mpsc::Sender;
use uuid::Uuid;

#[get("/")]
pub fn index() -> &'static str {
    "Hello, world!"
}

#[derive(Deserialize)]
pub struct TaskCommandRequest {
    id: String,
    command: String,
}

#[post("/apply", format = "json", data = "<request>")]
pub async fn apply(
    sender: &State<Sender<TaskCommand>>,
    request: Json<TaskCommandRequest>,
) -> Status {
    let parsed = match Uuid::parse_str(request.id.as_str()) {
        Ok(id) => id,
        Err(err) => {
            println!("failed to parse uuid {}", err);
            return Status::BadRequest;
        }
    };

    let cmd = match Command::from_string(request.command.as_str()) {
        Ok(cmd) => cmd,
        Err(err) => {
            println!("failed to parse cmd {}", err);
            return Status::BadRequest;
        }
    };

    match sender.send(TaskCommand::new(parsed, cmd)).await {
        Ok(_) => Status::Ok,
        Err(err) => {
            println!("failed to parse cmd {}", err);
            Status::InternalServerError
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::tracker::manager::{Command, TaskCommand};
    use crate::web::build::rocket;
    use rocket::http::{ContentType, Status};
    use rocket::local::asynchronous::Client;
    use tokio::sync::mpsc::channel;
    use tokio::sync::mpsc::error::TryRecvError;
    use uuid::Uuid;

    #[tokio::test]
    async fn successful_apply() {
        println!("{:?}", Uuid::new_v4().to_simple());
        let (cmd_send, mut cmd_receive) = channel::<TaskCommand>(1);

        let r = rocket(cmd_send);
        let client = Client::tracked(r).await.expect("valid rocket instance");
        let req = client
            .post("/apply")
            .header(ContentType::JSON)
            .body(r#"{"id": "a54a0fb9-25c9-4f73-ad82-0b7f30ca1ab6", "command": "stop"}"#);
        let response = req.dispatch().await;
        assert_eq!(response.status(), Status::Ok);
        let recv = cmd_receive.recv().await;
        assert!(recv.is_some());
        assert_eq!(
            recv.unwrap(),
            TaskCommand::new(
                Uuid::parse_str("a54a0fb9-25c9-4f73-ad82-0b7f30ca1ab6").unwrap(),
                Command::Stop
            )
        );
    }

    #[tokio::test]
    async fn invalid_uuid_apply() {
        let (cmd_send, mut cmd_receive) = channel::<TaskCommand>(1);

        let r = rocket(cmd_send);
        let client = Client::tracked(r).await.expect("valid rocket instance");
        let req = client
            .post("/apply")
            .header(ContentType::JSON)
            .body(r#"{"id": "94c3816b-c4f5-4748-bb96-", "command": "stop"}"#);
        let response = req.dispatch().await;
        assert_eq!(response.status(), Status::BadRequest);
        assert_eq!(Err(TryRecvError::Empty), cmd_receive.try_recv());
    }

    #[tokio::test]
    async fn invalid_command_apply() {
        let (cmd_send, mut cmd_receive) = channel::<TaskCommand>(1);

        let r = rocket(cmd_send);
        let client = Client::tracked(r).await.expect("valid rocket instance");
        let req = client
            .post("/apply")
            .header(ContentType::JSON)
            .body(r#"{"id": "94c3816b-c4f5-4748-bb96-3b8609f70b97", "command": "invalid"}"#);
        let response = req.dispatch().await;
        assert_eq!(response.status(), Status::BadRequest);
        assert_eq!(Err(TryRecvError::Empty), cmd_receive.try_recv());
    }
}
