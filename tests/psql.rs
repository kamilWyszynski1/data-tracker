extern crate datatracker_rust;
use datatracker_rust::connector::factory::getter_from_task_input;
use datatracker_rust::core::manager::TaskCommand;
use datatracker_rust::core::task::{TaskInput, TrackingTask};
use datatracker_rust::core::tracker::Tracker;
use datatracker_rust::core::types::Direction;
use datatracker_rust::lang::engine::Definition;
use datatracker_rust::lang::lexer::EvalForest;
use datatracker_rust::persistance::interface::Db;
use datatracker_rust::persistance::sqlite::{establish_connection, SqliteClient};
use datatracker_rust::wrap::StdoutAPI;
use tokio::sync::broadcast;
use tokio::sync::mpsc::channel;
use tokio::time::{sleep, Duration};

#[macro_use]
extern crate diesel_migrations;
embed_migrations!("migrations");

#[macro_use]
extern crate log;

#[tokio::test]
async fn test_psql_connector() {
    env_logger::init();
    let (shutdown_notify, shutdown_recv) = broadcast::channel(1);
    let (tt_send, receive) = channel::<TrackingTask>(10);
    let (_, cmd_receive) = channel::<TaskCommand>(10);

    let api = StdoutAPI::default();

    let conn = establish_connection();
    embedded_migrations::run_with_output(&conn, &mut std::io::stdout()).unwrap();
    let pers = SqliteClient::new(conn);

    let db = Db::new(Box::new(pers));

    let mut tracker = Tracker::new(
        api,
        db.clone(),
        receive,
        shutdown_recv,
        shutdown_notify.clone(),
        cmd_receive,
    );
    info!("initialized");

    tokio::task::spawn(async move {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                // The shutdown signal has been received.
                shutdown_notify.send(()).unwrap();
                info!("shutting down");
            }
        }
    });

    tokio::task::spawn(async move {
        tracker.start().await;
    });

    let ef = EvalForest::from_definition(&Definition::new(vec![String::from(
        "DEFINE(OUT, EXTRACT(GET(IN), 1))",
    )]));
    let input = TaskInput::PSQL {
        host: String::from("localhost"),
        port: 5432,
        user: String::from("postgres"),
        password: String::from("password"),
        query: String::from("SELECT value FROM test_table"),
        db: String::from("test"),
    };
    let empty_string = String::from("test");
    let tt = TrackingTask::new(
        empty_string.clone(),
        empty_string,
        String::from("A1"),
        Direction::Horizontal,
        getter_from_task_input(&input),
        std::time::Duration::from_secs(1),
    )
    .with_eval_forest(ef)
    .with_input(input);
    tt_send.send(tt).await.unwrap();

    sleep(Duration::from_millis(10000)).await;
}
