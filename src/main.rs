extern crate datatracker_rust;
use datatracker_rust::core::channels::ChannelsManager;
use datatracker_rust::core::manager::TaskCommand;
use datatracker_rust::core::task::TrackingTask;
use datatracker_rust::core::tracker::Tracker;
use datatracker_rust::core::types::{Direction, Hook};
use datatracker_rust::lang::engine::Definition;
use datatracker_rust::lang::eval::EvalForest;
use datatracker_rust::persistance::interface::Db;
use datatracker_rust::persistance::sqlite::{establish_connection, SqliteClient};
use datatracker_rust::server::build::rocket;
use datatracker_rust::server::proto::StatsService;
use datatracker_rust::server::task::TaskKindRequest;
use datatracker_rust::stats::stats_server::StatsServer;
use datatracker_rust::wrap::StdoutAPI;
use diesel_migrations::embed_migrations;
use tokio::join;
use tokio::sync::broadcast;
use tokio::sync::mpsc::channel;
use tonic::transport::Server;

#[macro_use]
extern crate diesel_migrations;

embed_migrations!("migrations");

#[macro_use]
extern crate log;

#[tokio::main]
async fn main() {
    env_logger::init();
    let (shutdown_notify, shutdown_recv) = broadcast::channel(1);
    let (tt_send, receive) = channel::<TrackingTask>(10);
    let (cmd_send, cmd_receive) = channel::<TaskCommand>(10);

    let api = StdoutAPI::default();
    let pers = SqliteClient::new(establish_connection());
    let db = Db::new(Box::new(pers));
    let channels_manager = ChannelsManager::default();

    let mut tracker = Tracker::new(
        api,
        db.clone(),
        channels_manager,
        receive,
        shutdown_recv,
        shutdown_notify.clone(),
        cmd_receive,
    );
    info!("initialized");

    let ef = EvalForest::from_definition(&Definition::new(vec![String::from(
        "DEFINE(OUT, EXTRACT(GET(IN), 0))",
    )]));

    tt_send
        .send(
            TrackingTask::new(
                String::from("test spreadsheet"),
                String::from("test sheet"),
                String::from("A1"),
                Direction::Horizontal,
                None,
                TaskKindRequest::Triggered(Hook::Kafka {
                    topic: String::from("test_topic"),
                    group_id: String::from("1"),
                    brokers: String::from("localhost:9092"),
                }),
            )
            .with_eval_forest(ef),
        )
        .await
        .unwrap();

    tokio::task::spawn(async move {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                // The shutdown signal has been received.
                shutdown_notify.send(()).unwrap();
                info!("shutting down");
            }
        }
    });

    let start = tokio::task::spawn(async move {
        tracker.start().await;
    });

    let addr = "[::1]:10000".parse().unwrap();
    let stats = StatsService::new(db.clone());
    let svc = StatsServer::new(stats);
    let rpc_server_start = tokio::task::spawn(async move {
        Server::builder()
            .add_service(svc)
            .serve_with_shutdown(addr, shutdown())
            .await
            .unwrap();
    });

    let rocket = rocket(cmd_send, tt_send, db);
    let (_, _, _) = join!(rocket.launch(), start, rpc_server_start);
}

async fn shutdown() {
    tokio::signal::ctrl_c().await.unwrap()
}
