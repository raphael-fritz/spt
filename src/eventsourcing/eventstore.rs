//! In-Memory Event Store
//!
//! This module provides an implementation of the event store trait for a simple in-memory
//! cache. This is not an event store you should be using for production and we recommend
//! it is recommended that you only use this for testing/demonstration purposes.

use super::Event;
use super::Result;
use serde_json;
use std::fs::File;
use std::io::BufRead;
//use std::io::BufReader;
use super::uevents::UniqueEvent;
use chrono::{DateTime, Utc};
use std::io;
use std::io::Write;
use std::path::Path;
use std::sync::Mutex;

/// An simple, in-memory implementation of the event store trait
pub struct JSONEventStore {
    pub evts: Mutex<Vec<UniqueEvent>>,
}

pub trait EventStore {
    fn append(&self, evt: impl Event, stream: &str) -> Result<UniqueEvent>;
}

impl JSONEventStore {
    /// Creates a new in-memory event store. The resulting store is thread-safe.
    pub fn new() -> JSONEventStore {
        JSONEventStore {
            evts: Mutex::new(Vec::<UniqueEvent>::new()),
        }
    }

    pub fn len(&self) -> usize {
        let guard = self.evts.lock().unwrap();
        guard.len()
    }

    pub fn from_file<P: AsRef<Path> + ?Sized + std::convert::AsRef<std::ffi::OsStr>>(
        path: &P,
    ) -> std::result::Result<JSONEventStore, crate::types::SPTError> {
        let file = File::open(path)?;
        let lines = io::BufReader::new(file).lines();
        let mut events = Vec::<UniqueEvent>::new();
        for line in lines {
            let line = line?;
            let event: UniqueEvent = serde_json::from_str(&line)?;
            events.push(event);
        }

        Ok(JSONEventStore {
            evts: Mutex::new(events),
        })
    }

    pub fn save_events<P: AsRef<Path> + ?Sized>(
        &self,
        path: &P,
    ) -> std::result::Result<(), crate::types::SPTError> {
        let guard = self.evts.lock().unwrap();
        let events: Vec<UniqueEvent> = guard.iter().cloned().collect();
        let file = File::create(path)?;
        let mut file = io::BufWriter::new(file);
        for event in events {
            let event = serde_json::to_string(&event)?;
            write!(file, "{}\n", event)?;
        }
        Ok(())
    }
}

impl EventStore for JSONEventStore {
    /// Appends an event to the in-memory store
    fn append(&self, evt: impl Event, _stream: &str) -> Result<UniqueEvent> {
        let mut guard = self.evts.lock().unwrap();
        let event = UniqueEvent::from(evt);
        guard.push(event.clone());
        Ok(event)
    }
}

#[allow(dead_code)]
impl JSONEventStore {
    pub fn get_all<E: Event + std::convert::From<UniqueEvent>>(&self, id: String) -> Vec<E> {
        let guard = self.evts.lock().unwrap();
        let matches = guard
            .iter()
            .map(|evt| evt.clone().into())
            .filter(|evt: &E| evt.event_origin_id() == id)
            .collect();
        matches
    }

    pub fn get_from<E: Event + std::convert::From<UniqueEvent>>(
        &self,
        id: String,
        start: DateTime<Utc>,
    ) -> Result<Vec<E>> {
        let guard = self.evts.lock().unwrap();
        let matches = guard
            .iter()
            .filter(|evt| evt.event_time >= start)
            .map(|evt| evt.clone().into())
            .filter(|evt: &E| evt.event_origin_id() == id)
            .collect();
        Ok(matches)
    }

    pub fn get_range<E: Event + std::convert::From<UniqueEvent>>(
        &self,
        id: String,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<E>> {
        let guard = self.evts.lock().unwrap();
        let matches = guard
            .iter()
            .filter(|evt| evt.event_time >= start && evt.event_time <= end)
            .map(|evt| evt.clone().into())
            .filter(|evt: &E| evt.event_origin_id() == id)
            .collect();
        Ok(matches)
    }
}
