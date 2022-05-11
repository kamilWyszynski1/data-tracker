use crate::core::task::InputData;
use rdkafka::{
    consumer::{stream_consumer::StreamConsumer, Consumer},
    ClientConfig, Message,
};
use tokio::sync::{broadcast, mpsc::Sender};

#[derive(Debug, Clone)]
/// Configuration for kafka client.
pub struct KafkaConfig {
    pub topic: String,
    pub group_id: String,
    pub brokers: String,
}

/// Starts consuming messages from kafka. Client is configured using KafkaConfig.  
pub async fn consume_topic(
    cfg: KafkaConfig,
    sender: Sender<InputData>,
    shutdown: &mut broadcast::Receiver<()>,
) {
    let consumer: StreamConsumer = ClientConfig::new()
        .set("group.id", &cfg.group_id)
        .set("bootstrap.servers", &cfg.brokers)
        .set("session.timeout.ms", "6000")
        .set("enable.auto.commit", "true")
        .set("api.version.request.timeout.ms", "6000")
        .set("api.version.request", "true")
        .set("broker.version.fallback", "2.3.1")
        .create()
        .expect("Consumer creation failed");

    consumer
        .subscribe(&[&cfg.topic])
        .expect("Can't subscribe to specified topic");
    debug!("consumer created");

    loop {
        tokio::select! {
            _ = shutdown.recv() => {
                debug!("consume_topic: closing");
                return;
            }
            msg = consumer.recv() => {
                match msg {
                    Err(e) => warn!("Kafka error: {}", e),
                    Ok(m) => {
                        let payload = match m.payload_view::<str>() {
                            None => "",
                            Some(Ok(s)) => s,
                            Some(Err(e)) => {
                                warn!("Error while deserializing message payload: {:?}", e);
                                ""
                            }
                        };
                        debug!("consume_topic: received message: {:?}, {}", m, payload);

                        match InputData::from_str(payload) {
                            Ok(id) => sender
                                .send(id)
                                .await
                                .expect("Can't send InputData from consume_topic"),
                            Err(e) => warn!("Error while deserializing message payload: {:?}", e),
                        }
                    }
                }
            }
        }
    }
}
