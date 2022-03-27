use datatracker_rust::data::kafka::{KConsumer, Producer};

#[tokio::main]
async fn main() {
    env_logger::init();

    let producer = Producer::new("localhost:9092");
    let consumer = KConsumer::new("localhost:9092", "1");

    producer
        .send(
            String::from("key"),
            String::from("payload"),
            String::from("test"),
        )
        .await;
    consumer
        .consume("test", |payload| {
            let payload_clone = payload;
            producer.send(String::from("key2"), payload_clone, String::from("topic2"))
        })
        .await;
}
