//! Unique Events Implementation

use super::Event;
use chrono::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UniqueEvent {
    pub event_type_version: String,
    pub origin_id: String,
    pub event_id: String,
    pub event_time: DateTime<Utc>,
    pub data: serde_json::Value,
}

impl<E> From<E> for UniqueEvent
where
    E: Event,
{
    fn from(source: E) -> Self {
        UniqueEvent {
            event_type_version: source.event_type_version().to_owned(),
            origin_id: source.event_origin_id(),
            event_id: Uuid::new_v4().hyphenated().to_string(),
            event_time: Utc::now(),
            data: serde_json::to_value(&source)
                .expect("Event implements Serialize so this should never panic."),
        }
    }
}
