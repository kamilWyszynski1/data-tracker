use datatracker_rust::{
    connector::kafka::{consume_topic, KafkaConfig},
    core::{
        channels::ChannelsManager,
        manager::TaskCommand,
        task::{InputData, TrackingTask},
        tracker::Tracker,
        types::{Direction, Hook},
    },
    lang::process::{Definition, Process},
    persistance::{in_memory::InMemoryPersistance, interface::Db},
    server::task::TaskKindRequest,
    wrap::TestAPI,
};
use rdkafka::{
    producer::{BaseProducer, BaseRecord},
    ClientConfig,
};
use serde_json::Value;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc::channel};

#[macro_use]
extern crate log;

pub fn can_be_run() -> bool {
    match std::env::var("INTEGRATION") {
        Ok(val) => val == *"1",
        Err(_) => false,
    }
}

#[tokio::test]
async fn test_kafka_connector() {
    if !can_be_run() {
        println!("skipped");
        return;
    }
    env_logger::try_init().ok();

    let cfg = KafkaConfig {
        topic: String::from("test_topic"),
        group_id: String::from("0"),
        brokers: String::from("localhost:9092"),
    };

    let producer: BaseProducer = ClientConfig::new()
        .set("bootstrap.servers", &cfg.brokers)
        .set("message.timeout.ms", "5000")
        .create()
        .expect("Producer creation error");

    let input_data = InputData::Vector(vec![
        InputData::String(String::from("test")),
        InputData::Json(Value::Bool(true)),
    ]);
    let payload = input_data
        .try_to_string()
        .expect("failed serialize InputData");

    let (sender, mut receiver) = channel::<InputData>(1);
    let (sender_shutdown, mut shutdown) = broadcast::channel(1);
    tokio::task::spawn(async move {
        debug!("start consuming");
        consume_topic(cfg, sender, &mut shutdown).await
    });

    tokio::time::sleep(Duration::from_secs(2)).await;
    producer
        .send(BaseRecord::to("test_topic").payload(&payload).key("key"))
        .expect("Failed to send kafka message");
    debug!("looping");

    loop {
        tokio::select! {
            n = receiver.recv() => {
                if let Some(n) = n {
                    println!("{:?}", n);
                    assert_eq!(n, input_data);
                    break;
                }
            }
        }
    }
    drop(sender_shutdown);
}

#[tokio::test]
async fn test_kafka_connector_whole_flow() {
    if !can_be_run() {
        println!("skipped");
        return;
    }

    env_logger::try_init().ok();

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
        vec![Definition::new(vec!["DEFINE(OUT, EXTRACT(GET(IN), 0))"])],
        None,
    );

    let empty_string = String::from("test");
    let tt = TrackingTask::new(
        empty_string.clone(),
        empty_string,
        String::from("A1"),
        Direction::Horizontal,
        None,
        TaskKindRequest::Triggered(Hook::Kafka {
            topic: String::from("test_topic"),
            group_id: String::from("0"),
            brokers: String::from("localhost:9092"),
        }),
    )
    .with_process(process);

    tt_send.send(tt).await.unwrap();

    // populate data to kafka.
    tokio::time::sleep(Duration::from_secs(2)).await;

    let cfg = KafkaConfig {
        topic: String::from("test_topic"),
        group_id: String::from("0"),
        brokers: String::from("localhost:9092"),
    };
    let producer: BaseProducer = ClientConfig::new()
        .set("bootstrap.servers", &cfg.brokers)
        .set("message.timeout.ms", "5000")
        .create()
        .expect("Producer creation error");

    let input_data = InputData::Vector(vec![
        InputData::String(String::from("test")),
        InputData::Json(Value::Bool(true)),
    ]);

    let payload = input_data
        .try_to_string()
        .expect("failed serialize InputData");
    producer
        .send(BaseRecord::to("test_topic").payload(&payload).key("key"))
        .expect("Failed to send kafka message");

    loop {
        if let Some(values) = test_receiver.recv().await {
            println!("{:?}", values);
            assert_eq!(values[0][0], String::from(r#"String("test")"#));
            return;
        }
    }
}

#[tokio::test]
async fn test_kafka_connector_shutdown() {
    if !can_be_run() {
        println!("skipped");
        return;
    }

    let cfg = KafkaConfig {
        topic: String::from("test_topic"),
        group_id: String::from("1"),
        brokers: String::from("localhost:9092"),
    };

    let (sender, _) = channel::<InputData>(1);
    let (sender_shutdown, mut shutdown) = broadcast::channel(1);
    tokio::task::spawn(async move {
        sender_shutdown
            .send(())
            .expect("failed to send message to sender_shutdown")
    });
    consume_topic(cfg, sender, &mut shutdown).await;
}
