use std::sync::Arc;

use axum::{
    extract::{Json, State},
    routing::post,
    Router,
};
use prost::Message;
use rdkafka::{
    producer::{FutureProducer, FutureRecord},
    ClientConfig,
    util::Timeout,
    error::KafkaError,
    message::OwnedMessage,
};
use tonic::{transport::Server, Request, Response, Status};
use tower_http::trace::TraceLayer;

use ingestion_service::generated::{Empty, Event};
use ingestion_service::generated::ingestion_service_server::{
    IngestionService, IngestionServiceServer,
};
use ingestion_service::handle_event;

type AppState = Arc<FutureProducer>;

async fn send_to_kafka(producer: &FutureProducer, evt: &Event) {
    let mut buf = Vec::<u8>::new();
    evt.encode(&mut buf).unwrap();

    match producer
        .send(
            FutureRecord::to("events")
                .payload(&buf)
                .key(&evt.id),
            Timeout::Never,
        )
        .await
    {
        Ok((partition, offset)) => {
            println!(
                "Produced `{}` to partition {partition} @ offset {offset}",
                evt.id
            );
        }
        Err((e, _msg)) => {
            eprintln!("Kafka send error for `{}`: {e}", evt.id);
        }
    }
}

#[derive(Clone)]
pub struct IngestionSvc {
    producer: AppState,
}

impl IngestionSvc {
    fn new(p: FutureProducer) -> Self {
        Self {
            producer: Arc::new(p),
        }
    }
}

#[tonic::async_trait]
impl IngestionService for IngestionSvc {
    async fn send_event(
        &self,
        request: Request<Event>,
    ) -> Result<Response<Empty>, Status> {
        let evt = request.into_inner();
        handle_event(evt.clone()).await;
        send_to_kafka(&self.producer, &evt).await;
        Ok(Response::new(Empty {}))
    }
}

async fn rest_send_event(
    State(producer): State<AppState>,
    Json(evt): Json<Event>,
) -> Json<Empty> {
    handle_event(evt.clone()).await;
    send_to_kafka(&producer, &evt).await;
    Json(Empty {})
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let producer: FutureProducer = ClientConfig::new()
        .set("bootstrap.servers", "localhost:9092")
        .create()?;
    
    let app_state: AppState = Arc::new(producer.clone());
    let app = Router::new()
        .route("/v1/event", post(rest_send_event))
        .layer(TraceLayer::new_for_http())
        .with_state(app_state.clone());

    tokio::spawn(async move {
        let addr = "[::1]:8080".parse().unwrap();
        if let Err(e) = axum::Server::bind(&addr)
            .serve(app.into_make_service())
            .await
        {
            eprintln!("HTTP server error: {e}");
        }
    });
    
    let grpc_addr = "[::1]:50051".parse()?;
    println!("gRPC  listening on {grpc_addr}");
    println!("REST   listening on http://[::1]:8080");

    Server::builder()
        .add_service(IngestionServiceServer::new(IngestionSvc::new(
            producer,
        )))
        .serve(grpc_addr)
        .await?;

    Ok(())
}
