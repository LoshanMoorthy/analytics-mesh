pub mod generated {
    include!("generated/analytics.rs");
}

use chrono::Utc;
use generated::Event;

pub fn enrich(mut evt: Event) -> Event {
    let attrs = &mut evt.attributes;
    attrs
        .entry("geo_city".into())
        .or_insert("Berlin".into());
    attrs
        .entry("processed_ts".into())
        .or_insert(Utc::now().timestamp_millis().to_string());
    evt
}
