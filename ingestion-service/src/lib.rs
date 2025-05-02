pub mod generated {
    include!("generated/analytics.rs");
}

use generated::Event;

pub async fn handle_event(event: Event) {
    println!("Received event: {:?}", event);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_handle_event_logs() {
        let evt = Event {
            id: "test".into(),
            timestamp: 1_700_000_000_000,
            attributes: Default::default(),
            payload: Vec::new(),
        };
        handle_event(evt).await;
    }
}
