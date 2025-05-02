use prost::Message as ProstMessage;
use rdkafka::{
    consumer::{Consumer, StreamConsumer},
    producer::{FutureProducer, FutureRecord},
    util::Timeout,
    ClientConfig,
    Message as KafkaMessage,
};use tokio_stream::StreamExt;
use tracing::{info, error};
use tracing_subscriber::FmtSubscriber;

use enrichment_service::{enrich, generated::Event};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    FmtSubscriber::builder().with_target(false).init();
    
    let consumer: StreamConsumer = ClientConfig::new()
        .set("group.id", "enrichment-service")
        .set("bootstrap.servers", "localhost:9092")
        .set("enable.partition.eof", "false")
        .set("auto.offset.reset", "earliest")
        .create()?;
    consumer.subscribe(&["events"])?;
    
    let producer: FutureProducer = ClientConfig::new()
        .set("bootstrap.servers", "localhost:9092")
        .create()?;

    info!("⏳  waiting for messages from topic `events` …");

    let mut stream = consumer.stream();
    while let Some(Ok(msg)) = stream.next().await {
        let payload = match msg.payload() {
            Some(p) => p,
            None => { error!("👻 empty payload"); continue; }
        };
        let evt = match Event::decode(payload) {
            Ok(e) => e,
            Err(e) => { error!("decode error: {e}"); continue; }
        };
        info!("➡️  got {}", evt.id);

        let enriched = enrich(evt);

        let mut buf = Vec::new();
        enriched.encode(&mut buf)?;

        match producer.send(
            FutureRecord::to("enriched-events")
                .payload(&buf)
                .key(&enriched.id),
            Timeout::Never,
        ).await {
            Ok((part, off)) => info!("✅ sent `{}` to partition {part}@{off}", enriched.id),
            Err((e, _))     => error!("❌ send error: {e}"),
        }
    }
    Ok(())
}
