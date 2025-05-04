use metrics::{counter, histogram};
use metrics_exporter_prometheus::PrometheusBuilder;
use prost::Message;
use rdkafka::{
    config::ClientConfig,
    consumer::{Consumer, StreamConsumer},
    message::Message as KafkaMessage,
    producer::{FutureProducer, FutureRecord},
    util::Timeout,
};

use tonic::{transport::Server, Request, Response, Status};
use axum::{routing::post, Router, extract::Json, extract::State};
use tower_http::trace::TraceLayer;
use std::sync::Arc;
use dashmap::DashMap;
use tokio_stream::StreamExt;

pub mod generated {
    include!("generated/analytics.rs");
}
use generated::{Empty, Event};

type AppState = Arc<FutureProducer>;

async fn send_to_kafka(producer: &FutureProducer, evt: &Event) {
    println!("ingestion got event: {:?}", evt);
    
    counter!("events_ingested_total").increment(1);
    histogram!("event_payload_bytes").record(evt.payload.len() as f64);

    let mut buf = Vec::new();
    evt.encode(&mut buf).unwrap();

    if let Err((e, _)) = producer
        .send(
            FutureRecord::to("events")
                .payload(&buf)
                .key(&evt.id),
            Timeout::Never,
        )
        .await
    {
        eprintln!("Kafka send error for `{}`: {e}", evt.id);
    }
}

#[derive(Clone)]
pub struct IngestionSvc {
    producer: AppState,
}

#[tonic::async_trait]
impl generated::ingestion_service_server::IngestionService for IngestionSvc {
    async fn send_event(
        &self,
        request: Request<Event>,
    ) -> Result<Response<Empty>, Status> {
        let evt = request.into_inner();
        send_to_kafka(&self.producer, &evt).await;
        Ok(Response::new(Empty {}))
    }
}

async fn rest_send_event(
    State(producer): State<AppState>,
    Json(evt): Json<Event>,
) -> Json<Empty> {
    send_to_kafka(&producer, &evt).await;
    Json(Empty {})
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    PrometheusBuilder::new()
        .with_http_listener(([0, 0, 0, 0], 9100))
        .install_recorder()?;
    println!("ingestion metrics on http://0.0.0.0:9100/metrics");

    let brokers = std::env::var("BOOTSTRAP_SERVERS")
        .unwrap_or_else(|_| "kafka:9092".into());
    let producer: FutureProducer = ClientConfig::new()
        .set("bootstrap.servers", &brokers)
        .create()?;
    let shared = Arc::new(producer.clone());

    let http_app = Router::new()
        .route(
            "/v1/event",
            post(move |State(prod): State<AppState>, Json(evt): Json<Event>| {
                let prod = prod.clone();
                async move {
                    send_to_kafka(&prod, &evt).await;
                    Json(Empty {})
                }
            }),
        )
        .layer(TraceLayer::new_for_http())
        .with_state(shared.clone());

    tokio::spawn(async move {
        axum::Server::bind(&"0.0.0.0:8080".parse().unwrap())
            .serve(http_app.into_make_service())
            .await
            .unwrap();
    });
    println!("REST listening on http://0.0.0.0:8080");

    #[derive(Clone)]
    struct GrpcSvc {
        producer: AppState,
    }
    #[tonic::async_trait]
    impl generated::ingestion_service_server::IngestionService for GrpcSvc {
        async fn send_event(
            &self,
            request: tonic::Request<Event>,
        ) -> Result<tonic::Response<Empty>, tonic::Status> {
            let evt = request.into_inner();
            send_to_kafka(&self.producer, &evt).await;
            Ok(tonic::Response::new(Empty {}))
        }
    }

    let grpc_addr = "[::]:50051".parse()?;
    println!("gRPC listening on {}", grpc_addr);
    tonic::transport::Server::builder()
        .add_service(
            generated::ingestion_service_server::IngestionServiceServer::new(
                GrpcSvc { producer: shared },
            ),
        )
        .serve(grpc_addr)
        .await?;

    Ok(())
}
