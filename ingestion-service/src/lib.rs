pub mod generated {
    include!("generated/analytics.rs");
}

use prost::Message;
use generated::Event;
use rdkafka::{
    producer::{FutureProducer, FutureRecord},
    util::Timeout,
    error::KafkaError,
};
pub async fn handle_event(
    event: Event,
    producer: &FutureProducer,
) -> Result<(), KafkaError> {
    println!("Received event: {:?}", event);

    let mut buf = Vec::new();
    event.encode(&mut buf)
        .expect("Failed to encode Event"); // you can also `.map_err(|e| KafkaError::MessageProduction(e.to_string()))?`

    let result = producer
        .send(
            FutureRecord::to("events")
                .payload(&buf)
                .key(&event.id),
            Timeout::Never,
        )
        .await;

    match result {
        Ok((partition, offset)) => {
            println!(
                "✅ Produced `{}` to partition {} @ offset {}",
                event.id, partition, offset
            );
            Ok(())
        }
        Err((err, _owned_message)) => {
            eprintln!("❌ Failed to produce `{}`: {}", event.id, err);
            Err(err)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rdkafka::config::ClientConfig;
    use std::env;

    #[tokio::test]
    async fn test_handle_event_logs() {
        env::set_var("BOOTSTRAP_SERVERS", "localhost:0");
        let producer: FutureProducer = ClientConfig::new()
            .set("bootstrap.servers", "localhost:0")
            .create()
            .unwrap();

        let evt = Event {
            id: "test".into(),
            timestamp: 1_700_000_000_000,
            attributes: Default::default(),
            payload: Vec::new(),
        };
        let _ = handle_event(evt, &producer).await;
    }
}
