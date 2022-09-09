extern crate datatracker_rust;

use datatracker_rust::connector::factory::getter_from_task_input;
use datatracker_rust::connector::psql::{monitor_changes, PSQLConfig};
use datatracker_rust::core::channels::ChannelsManager;
use datatracker_rust::core::manager::TaskCommand;
use datatracker_rust::core::task::{InputData, TaskInput, TrackingTask};
use datatracker_rust::core::tracker::Tracker;
use datatracker_rust::core::types::{Direction, Hook};
use datatracker_rust::lang::engine::Definition;
use datatracker_rust::lang::process::Process;
use datatracker_rust::persistance::in_memory::InMemoryPersistance;
use datatracker_rust::persistance::interface::Db;
use datatracker_rust::server::task::TaskKindRequest;
use datatracker_rust::wrap::TestAPI;
use postgres::NoTls;
use tokio::sync::broadcast;
use tokio::sync::mpsc::channel;
use tokio::time::{sleep, Duration};
use tokio_postgres::Client;
#[macro_use]
extern crate log;

pub fn can_be_run() -> bool {
    match std::env::var("INTEGRATION") {
        Ok(val) => val == *"1",
        Err(_) => false,
    }
}

#[tokio::test]
async fn test_psql_connector() {
    if !can_be_run() {
        println!("skipped");
        return;
    }

    env_logger::try_init().ok();
    let (shutdown_notify, shutdown_recv) = broadcast::channel(1);
    let (tt_send, receive) = channel::<TrackingTask>(10);
    let (_, cmd_receive) = channel::<TaskCommand>(10);

    let (api, mut receiver) = TestAPI::new();

    let pers = InMemoryPersistance::new();

    let (client, psql_cfg) = prep_psql().await;
    client
        .execute("insert into test_table(id, value) values (1, 'test')", &[])
        .await
        .unwrap();

    let db = Db::new(Box::new(pers));

    let mut tracker = Tracker::new(
        api,
        db.clone(),
        ChannelsManager::default(),
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

    let process = Process::new(
        "main process",
        vec![Definition::new(vec![String::from("DEFINE(OUT, GET(IN))")])],
        None,
    );

    let input = TaskInput::PSQL {
        host: psql_cfg.host,
        port: psql_cfg.port,
        user: psql_cfg.user,
        password: psql_cfg.password,
        query: String::from("SELECT value FROM test_table WHERE id = 1"),
        db: psql_cfg.db,
    };
    let empty_string = String::from("test");
    let tt = TrackingTask::new(
        empty_string.clone(),
        empty_string,
        String::from("A1"),
        Direction::Horizontal,
        getter_from_task_input(&input),
        TaskKindRequest::Ticker { interval_secs: 1 },
    )
    .with_process(process)
    .with_input(input);

    tt_send.send(tt).await.unwrap();

    sleep(Duration::from_millis(500)).await;
    loop {
        tokio::select! {
            n = receiver.recv() => {
                match n {
                    Some(n) => {
                        assert_eq!(n[0][0], String::from(r#"String("test")"#));
                        return;
                    },
                    None =>()
                }
            }
        }
    }
}

#[tokio::test]
async fn test_changes_monitor() {
    if !can_be_run() {
        println!("skipped");
        return;
    }

    env_logger::try_init().ok();

    let (sender, mut receiver) = channel::<InputData>(1);
    let (shutdown_sender, shutdown_receiver) = broadcast::channel(1);
    let (client, psql_cfg) = prep_psql().await;
    tokio::task::spawn(async { monitor_changes(psql_cfg, sender, shutdown_receiver).await });
    tokio::task::spawn(async move {
        tokio::time::sleep(Duration::from_millis(500)).await;
        client
            .execute("insert into test_table(id, value) values (1, 'test')", &[])
            .await
            .unwrap();
    });

    loop {
        tokio::select! {
            n = receiver.recv() => {
                match n {
                    Some(n) => {
                        println!("{:?}", n);
                        assert_eq!(n, InputData::String(String::from(r#"{"id":1,"value":"test"}"#)));
                        break
                    },
                    None =>()
                }
            }
        }
    }
    drop(shutdown_sender);
}

#[tokio::test]
async fn test_changes_monitor_whole_flow() {
    if !can_be_run() {
        println!("skipped");
        return;
    }

    env_logger::try_init().ok();

    let (client, psql_cfg) = prep_psql().await;
    let (shutdown_notify, shutdown_recv) = broadcast::channel(1);
    let (tt_send, receive) = channel::<TrackingTask>(10);
    let (_, cmd_receive) = channel::<TaskCommand>(10);

    let (api, mut test_receiver) = TestAPI::new();
    let pers = InMemoryPersistance::new();
    let db = Db::new(Box::new(pers));

    let mut tracker = Tracker::new(
        api,
        db.clone(),
        ChannelsManager::default(),
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

    let process = Process::new(
        "main process",
        vec![Definition::new(vec![
            String::from("DEFINE(J, JSON(GET(IN)))"),
            String::from("DEFINE(OUT, EXTRACT(GET(J), id))"),
        ])],
        None,
    );

    let empty_string = String::from("test");
    let tt = TrackingTask::new(
        empty_string.clone(),
        empty_string,
        String::from("A1"),
        Direction::Horizontal,
        None,
        TaskKindRequest::Triggered(Hook::PSQL {
            host: psql_cfg.host,
            port: psql_cfg.port,
            user: psql_cfg.user,
            password: psql_cfg.password,
            db: psql_cfg.db,
            channel: psql_cfg.channel_name.as_ref().unwrap().to_string(),
        }),
    )
    .with_process(process);

    tt_send.send(tt).await.unwrap();

    tokio::task::spawn(async move {
        tokio::time::sleep(Duration::from_millis(500)).await;
        client
            .execute("insert into test_table(id, value) values (1, 'test')", &[])
            .await
            .unwrap();
    });

    loop {
        match test_receiver.recv().await {
            Some(values) => {
                println!("{:?}", values);
                assert_eq!(values[0][0], String::from("Int(1)"));
                return;
            }
            None => (),
        }
    }
}

async fn prep_psql() -> (Client, PSQLConfig) {
    let psql_cfg = PSQLConfig::new(
        String::from("localhost"),
        5432,
        String::from("postgres"),
        String::from("password"),
        String::from("postgres"),
        Some(String::from("test_channel")),
    );

    let (client, connection) = tokio_postgres::connect(psql_cfg.to_conn_str().as_str(), NoTls)
        .await
        .unwrap();
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });
    prepare_psql_db(&client).await;
    (client, psql_cfg)
}

async fn prepare_psql_db(client: &Client) {
    client
        .batch_execute(
            r#"
            drop table test_table;

            create table test_table
            (
                id    int primary key,
                value text
            );

            CREATE OR REPLACE FUNCTION notify_insert()
            RETURNS trigger AS
            $$
            DECLARE
            BEGIN
                PERFORM pg_notify(
                        CAST('test_channel' AS text),
                        row_to_json(NEW)::text);
                RETURN NEW;
            END;
            $$ LANGUAGE plpgsql;
            
            CREATE TRIGGER notify_insert
                AFTER INSERT
                ON test_table
                FOR EACH ROW
            EXECUTE PROCEDURE notify_insert();
        "#,
        )
        .await
        .unwrap();
}
