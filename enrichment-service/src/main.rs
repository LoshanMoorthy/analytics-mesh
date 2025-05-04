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

use metrics_exporter_prometheus::PrometheusBuilder;
use metrics::{counter, histogram};

use enrichment_service::{enrich, generated::Event};

use reqwest::Client;
use serde::Deserialize;
use dashmap::DashMap;
use std::sync::Arc;

#[derive(Deserialize)]
struct IpApiResponse {
    city: String,
    country: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    PrometheusBuilder::new()
        .with_http_listener(([0, 0, 0, 0], 9101))
        .install_recorder()?;
    println!("enrichment metrics on http://0.0.0.0:9101/metrics");

    FmtSubscriber::builder().with_target(false).init();

    let brokers = std::env::var("BOOTSTRAP_SERVERS")
        .unwrap_or_else(|_| "kafka:9092".into());
    
    let consumer: StreamConsumer = ClientConfig::new()
        .set("group.id", "enrichment-service")
        .set("bootstrap.servers", &brokers)
        .set("enable.partition.eof", "false")
        .set("auto.offset.reset", "earliest")
        .create()?;
    consumer.subscribe(&["events"])?;

    let producer: FutureProducer = ClientConfig::new()
        .set("bootstrap.servers", &brokers)
        .create()?;

    let http_client = Client::new();
    let geo_cache: Arc<DashMap<String, (String, String)>> = Arc::new(DashMap::new());

    info!("waiting for messages from topic events …");

    let mut stream = consumer.stream();
    while let Some(Ok(msg)) = stream.next().await {
        let payload = msg.payload().unwrap_or(&[]);
        let evt = match Event::decode(payload) {
            Ok(e) => e,
            Err(e) => { eprintln!("decode error: {}", e); continue; }
        };
        println!("got {}", evt.id);

        counter!("events_enriched_total").increment(1);
        histogram!("enriched_payload_bytes").record(evt.payload.len() as f64);
        
        let ip = evt.attributes.get("ip").cloned().unwrap_or_default();
        let (city, country) = if ip.is_empty() {
            ("unknown".into(), "unknown".into())
        } else if let Some(cached) = geo_cache.get(&ip) {
            cached.clone()
        } else {
            let resp_result = http_client
                .get(format!("http://ip-api.com/json/{}", ip))
                .send()
                .await;

            match resp_result {
                Ok(resp) => {
                    match resp.json::<IpApiResponse>().await {
                        Ok(info) => {
                            let tuple = (info.city.clone(), info.country.clone());
                            geo_cache.insert(ip.clone(), tuple.clone());
                            tuple
                        }
                        Err(_) => ("error".into(), "error".into()),
                    }
                }
                Err(_) => ("error".into(), "error".into()),
            }
        };

        let mut enriched = evt.clone();
        enriched.attributes.insert("geo_city".into(), city);
        enriched.attributes.insert("geo_country".into(), country);

        let mut buf = Vec::new();
        enriched.encode(&mut buf)?;
        producer.send(
            FutureRecord::to("enriched-events")
                .payload(&buf)
                .key(&enriched.id),
            Timeout::Never,
        ).await.expect("produce failed");

        println!("enriched {}", enriched.id);
    }

    Ok(())
}