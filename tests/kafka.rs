use datatracker_rust::{
    connector::kafka::{consume_topic, KafkaConfig},
    core::{
        channels::ChannelsManager,
        manager::TaskCommand,
        task::{InputData, TrackingTask},
        tracker::Tracker,
        types::{Direction, Hook},
    },
    lang::{engine::Definition, eval::EvalForest},
    persistance::{in_memory::InMemoryPersistance, interface::Db},
    server::task::TaskKindRequest,
    wrap::TestAPI,
};
use rdkafka::{
    producer::{FutureProducer, FutureRecord},
    ClientConfig,
};
use serde_json::Value;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc::channel};

#[macro_use]
extern crate log;

pub fn can_be_run() -> bool {
    match std::env::var("INTEGRATION") {
        Ok(val) => return val == String::from("1"),
        Err(_) => false,
    }
}

#[tokio::test]
async fn test_kafka_connector() {
    if !can_be_run() {
        println!("skipped");
        return;
    }
    env_logger::try_init();

    let cfg = KafkaConfig {
        topic: String::from("test_topic"),
        group_id: String::from("1"),
        brokers: String::from("localhost:9092"),
    };
    let producer: &FutureProducer = &ClientConfig::new()
        .set("bootstrap.servers", &cfg.brokers)
        .set("message.timeout.ms", "5000")
        .create()
        .expect("Producer creation error");

    let input_data = InputData::Vector(vec![
        InputData::String(String::from("test")),
        InputData::Json(Value::Bool(true)),
    ]);
    let payload = input_data.to_str().expect("failed serialize InputData");
    let fr = FutureRecord::to("test_topic").payload(&payload).key("key");

    let (sender, mut receiver) = channel::<InputData>(1);
    let (sender_shutdown, mut shutdown) = broadcast::channel(1);
    tokio::task::spawn(async move { consume_topic(cfg, sender, &mut shutdown).await });

    tokio::time::sleep(Duration::from_secs(2)).await;
    producer
        .send(fr, Duration::from_secs(0))
        .await
        .expect("Failed to send kafka message");
    debug!("looping");

    loop {
        tokio::select! {
            n = receiver.recv() => {
                match n {
                    Some(n) => {
                        println!("{:?}", n);
                        assert_eq!(n, input_data);
                        break;
                    },
                    None =>()
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

    env_logger::try_init();

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

    let ef = EvalForest::from_definition(&Definition::new(vec![String::from(
        "DEFINE(OUT, EXTRACT(GET(IN), 0))",
    )]));

    let empty_string = String::from("test");
    let tt = TrackingTask::new(
        empty_string.clone(),
        empty_string,
        String::from("A1"),
        Direction::Horizontal,
        None,
        TaskKindRequest::Triggered(Hook::Kafka {
            topic: String::from("test_topic"),
            group_id: String::from("1"),
            brokers: String::from("localhost:9092"),
        }),
    )
    .with_eval_forest(ef);

    tt_send.send(tt).await.unwrap();

    // populate data to kafka.
    tokio::time::sleep(Duration::from_secs(2)).await;

    let cfg = KafkaConfig {
        topic: String::from("test_topic"),
        group_id: String::from("0"),
        brokers: String::from("localhost:9092"),
    };
    let producer: &FutureProducer = &ClientConfig::new()
        .set("bootstrap.servers", &cfg.brokers)
        .set("message.timeout.ms", "5000")
        .create()
        .expect("Producer creation error");

    let input_data = InputData::Vector(vec![
        InputData::String(String::from("test")),
        InputData::Json(Value::Bool(true)),
    ]);
    let payload = input_data.to_str().expect("failed serialize InputData");
    let fr = FutureRecord::to("test_topic").payload(&payload).key("key");
    producer
        .send(fr, Duration::from_secs(0))
        .await
        .expect("Failed to send kafka message");

    loop {
        match test_receiver.recv().await {
            Some(values) => {
                println!("{:?}", values);
                assert_eq!(values[0][0], String::from(r#"String("test")"#));
                return;
            }
            None => (),
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
