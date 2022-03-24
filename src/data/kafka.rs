use rdkafka::config::{ClientConfig, RDKafkaLogLevel};
use rdkafka::consumer::{CommitMode, Consumer, StreamConsumer};
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::Message;
use std::future::Future;
use std::time::Duration;

/// Wrapper for kafka client.
pub struct Producer {
    producer: FutureProducer,
}

impl Producer {
    pub fn new(host: &str) -> Self {
        Self {
            producer: ClientConfig::new()
                .set("bootstrap.servers", host)
                .set("message.timeout.ms", "5000")
                .create()
                .expect("Producer creation error"),
        }
    }

    /// Sends message to wanted topic asynchronously.
    pub async fn send(&self, key: String, payload: String, topic: String) {
        let delivery_status = self
            .producer
            .send(
                FutureRecord::to(topic.as_str())
                    .payload(payload.as_str())
                    .key(key.as_str()),
                Duration::from_secs(0),
            )
            .await;
        info!(
            "Delivery status for message key: {} received: {:?}",
            key, delivery_status,
        );
    }
}

pub struct KConsumer {
    consumer: StreamConsumer,
}

impl KConsumer {
    pub fn new(host: &str, group_id: &str) -> Self {
        KConsumer {
            consumer: ClientConfig::new()
                .set("group.id", group_id)
                .set("bootstrap.servers", host)
                .set("enable.partition.eof", "false")
                .set("session.timeout.ms", "6000")
                .set("enable.auto.commit", "true")
                .set_log_level(RDKafkaLogLevel::Debug)
                .create()
                .expect("Consumer creation error"),
        }
    }

    /// Consumes messages from one topic and calls PayloadHandleFn on messages' payload.
    pub async fn consume<Fut>(&self, topic: &str, payload_fn: impl Fn(String) -> Fut)
    where
        Fut: Future<Output = ()>,
    {
        println!("here1");

        self.consumer
            .subscribe(&vec![topic])
            .expect("Can't subscribe to specified topics");
        println!("here2");

        loop {
            match self.consumer.recv().await {
                Err(err) => error!("{}", err),
                Ok(m) => {
                    let payload = match m.payload_view::<str>() {
                        None => "",
                        Some(Ok(s)) => s,
                        Some(Err(e)) => {
                            info!("Error while deserializing message payload: {:?}", e);
                            ""
                        }
                    };
                    info!("key: '{:?}', payload: '{}', topic: {}, partition: {}, offset: {}, timestamp: {:?}",
                          m.key(), payload, m.topic(), m.partition(), m.offset(), m.timestamp());
                    payload_fn(payload.to_string()).await;

                    self.consumer.commit_message(&m, CommitMode::Async).unwrap();
                }
            }
        }
    }
}
