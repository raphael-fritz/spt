//! In-Memory Event Store
//!
//! This module provides an implementation of the event store trait for a simple in-memory
//! cache. This is not an event store you should be using for production and we recommend
//! it is recommended that you only use this for testing/demonstration purposes.

use chrono::prelude::*;
use eventsourcing::eventstore::EventStore;
use eventsourcing::CloudEvent;
use eventsourcing::Event;
use eventsourcing::Result;
use serde_json;
use std::fs::File;
use std::io::BufRead;
//use std::io::BufReader;
use std::io;
use std::io::Write;
use std::path::Path;
use std::sync::Mutex;

/// An simple, in-memory implementation of the event store trait
pub struct JSONEventStore {
    pub evts: Mutex<Vec<CloudEvent>>,
}

impl JSONEventStore {
    /// Creates a new in-memory event store. The resulting store is thread-safe.
    pub fn new() -> JSONEventStore {
        JSONEventStore {
            evts: Mutex::new(Vec::<CloudEvent>::new()),
        }
    }

    pub fn from_file<P: AsRef<Path> + ?Sized>(path: &P) -> JSONEventStore {
        let file = File::open(path).unwrap();
        let lines = io::BufReader::new(file).lines();
        let mut events = Vec::<CloudEvent>::new();
        for line in lines {
            let line = line.unwrap();
            let event: CloudEvent = serde_json::from_str(&line).unwrap();
            events.push(event);
        }

        JSONEventStore {
            evts: Mutex::new(events),
        }
    }

    pub fn save_events<P: AsRef<Path> + ?Sized>(&self, path: &P) {
        let guard = self.evts.lock().unwrap();
        let events: Vec<CloudEvent> = guard.iter().cloned().collect();
        let file = File::create(path).unwrap();
        let mut file = io::BufWriter::new(file);
        for event in events {
            let event = serde_json::to_string(&event).unwrap();
            write!(file, "{}\n", event).unwrap();
        }
    }
}

impl EventStore for JSONEventStore {
    /// Appends an event to the in-memory store
    fn append(&self, evt: impl Event, _stream: &str) -> Result<CloudEvent> {
        let mut guard = self.evts.lock().unwrap();
        let cloud_event = CloudEvent::from(evt);
        guard.push(cloud_event.clone());
        Ok(cloud_event)
    }
}

impl JSONEventStore {
    pub fn get_all(&self, event_type: &str) -> Result<Vec<CloudEvent>> {
        let guard = self.evts.lock().unwrap();
        let matches = guard
            .iter()
            .filter(|evt| evt.event_type == event_type)
            .cloned()
            .collect();

        Ok(matches)
    }

    pub fn get_from(&self, event_type: &str, start: DateTime<Utc>) -> Result<Vec<CloudEvent>> {
        let guard = self.evts.lock().unwrap();
        let matches = guard
            .iter()
            .filter(|evt| evt.event_type == event_type && evt.event_time >= start)
            .cloned()
            .collect();
        Ok(matches)
    }

    pub fn get_range(
        &self,
        event_type: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<CloudEvent>> {
        let guard = self.evts.lock().unwrap();
        let matches = guard
            .iter()
            .filter(|evt| {
                evt.event_type == event_type && evt.event_time >= start && evt.event_time <= end
            })
            .cloned()
            .collect();
        Ok(matches)
    }
}
