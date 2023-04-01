use crate::types;
use eventsourcing::{prelude::*, Aggregate, CloudEvent, Result};
use serde::{Deserialize, Serialize};
use std::fmt;

const DOMAIN_VERSION: &str = "1.0";
const EVENT_SOURCE: &str = "events://SPT";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum PlaylistEvent {
    CreatedPlaylist(types::Playlist),
    UpdatedDesciption(Option<String>),
    UpdatedName(String),
    RemovedTracks(types::PlaylistItems),
    AddedTracks(types::PlaylistItems),
    DeletedPlaylist(),
}
impl fmt::Display for PlaylistEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PlaylistEvent::CreatedPlaylist(_) => write!(f, "PlaylistEvent.CreatedPlaylist"),
            PlaylistEvent::UpdatedDesciption(_) => write!(f, "PlaylistEvent.UpdatedDesciption"),
            PlaylistEvent::UpdatedName(_) => write!(f, "PlaylistEvent.UpdatedName"),
            PlaylistEvent::AddedTracks(_) => write!(f, "PlaylistEvent.AddedTracks"),
            PlaylistEvent::RemovedTracks(_) => write!(f, "PlaylistEvent.RemovedTracks"),
            PlaylistEvent::DeletedPlaylist() => write!(f, "PlaylistEvent.DeletedPlaylist"),
        }
    }
}
impl Event for PlaylistEvent {
    fn event_type_version(&self) -> &str {
        DOMAIN_VERSION
    }
    fn event_type(&self) -> &str {
        match self {
            PlaylistEvent::CreatedPlaylist(_) => "PlaylistEvent.CreatedPlaylist",
            PlaylistEvent::UpdatedDesciption(_) => "PlaylistEvent.UpdatedDesciption",
            PlaylistEvent::AddedTracks(_) => "PlaylistEvent.AddedTracks",
            PlaylistEvent::RemovedTracks(_) => "PlaylistEvent.RemovedTracks",
            PlaylistEvent::UpdatedName(_) => "PlaylistEvent.UpdatedName",
            PlaylistEvent::DeletedPlaylist() => "PlaylistEvent.DeletedPlaylist",
        }
    }
    fn event_source(&self) -> &str {
        EVENT_SOURCE
    }
}
impl From<CloudEvent> for PlaylistEvent {
    fn from(evt: CloudEvent) -> Self {
        serde_json::from_value(evt.data).unwrap()
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum PlaylistCommand {
    CreatePlaylist(types::Playlist),
    UpdateDesciption(Option<String>),
    UpdateName(String),
    AddTracks(types::PlaylistItems),
    RemoveTracks(types::PlaylistItems),
    DeletePlaylist(),
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlaylistData {
    pub data: types::Playlist,
    pub generation: u64,
}
impl PlaylistData {
    pub fn new() -> PlaylistData {
        PlaylistData {
            data: types::Playlist::new(),
            generation: 0,
        }
    }
}
impl AggregateState for PlaylistData {
    fn generation(&self) -> u64 {
        self.generation
    }
}
pub struct PlaylistAggregate;
impl Aggregate for PlaylistAggregate {
    type Event = PlaylistEvent;
    type State = PlaylistData;
    type Command = PlaylistCommand;

    fn apply_event(state: &Self::State, evt: &Self::Event) -> Result<Self::State> {
        let state = match &*evt {
            PlaylistEvent::CreatedPlaylist(playlist) => PlaylistData {
                data: playlist.to_owned(),
                generation: state.generation + 1,
            },
            PlaylistEvent::UpdatedName(newname) => PlaylistData {
                data: types::Playlist {
                    collaborative: state.data.collaborative,
                    followers: state.data.followers,
                    public: state.data.public,
                    description: state.data.description.clone(),
                    id: state.data.id.clone(),
                    name: newname.to_owned(),
                    owner: state.data.owner.clone(),
                    tracks: state.data.tracks.clone(),
                },
                generation: state.generation + 1,
            },
            PlaylistEvent::UpdatedDesciption(newdes) => PlaylistData {
                data: types::Playlist {
                    collaborative: state.data.collaborative,
                    followers: state.data.followers,
                    public: state.data.public,
                    description: newdes.to_owned(),
                    id: state.data.id.clone(),
                    name: state.data.name.clone(),
                    owner: state.data.owner.clone(),
                    tracks: state.data.tracks.clone(),
                },
                generation: state.generation + 1,
            },
            PlaylistEvent::AddedTracks(tracks) => {
                let mut ntracks = state.data.tracks.clone();
                ntracks.0.append(&mut tracks.0.clone());
                PlaylistData {
                    data: types::Playlist {
                        collaborative: state.data.collaborative,
                        followers: state.data.followers,
                        public: state.data.public,
                        description: state.data.description.clone(),
                        id: state.data.id.clone(),
                        name: state.data.name.clone(),
                        owner: state.data.owner.clone(),
                        tracks: ntracks,
                    },
                    generation: state.generation + 1,
                }
            }
            PlaylistEvent::RemovedTracks(tracks) => {
                let mut ntracks = state.data.tracks.clone();
                for track in tracks.0.clone() {
                    ntracks.0.retain(|x| *x != track);
                }
                PlaylistData {
                    data: types::Playlist {
                        collaborative: state.data.collaborative,
                        followers: state.data.followers,
                        public: state.data.public,
                        description: state.data.description.clone(),
                        id: state.data.id.clone(),
                        name: state.data.name.clone(),
                        owner: state.data.owner.clone(),
                        tracks: ntracks,
                    },
                    generation: state.generation + 1,
                }
            }
            PlaylistEvent::DeletedPlaylist() => todo!(),
        };
        Ok(state)
    }
    fn handle_command(_state: &Self::State, cmd: &Self::Command) -> Result<Vec<Self::Event>> {
        // SHOULD DO: validate state and command
        // if validation passes...
        let evts = match cmd {
            PlaylistCommand::CreatePlaylist(playlist) => {
                vec![PlaylistEvent::CreatedPlaylist(playlist.to_owned())]
            }
            PlaylistCommand::UpdateName(newname) => {
                vec![PlaylistEvent::UpdatedName(newname.to_owned())]
            }
            PlaylistCommand::UpdateDesciption(newdes) => {
                vec![PlaylistEvent::UpdatedDesciption(newdes.to_owned())]
            }
            PlaylistCommand::AddTracks(tracks) => {
                vec![PlaylistEvent::AddedTracks(tracks.to_owned())]
            }
            PlaylistCommand::RemoveTracks(tracks) => {
                vec![PlaylistEvent::RemovedTracks(tracks.to_owned())]
            }
            PlaylistCommand::DeletePlaylist() => {
                vec![PlaylistEvent::DeletedPlaylist()]
            }
        };
        Ok(evts)
    }
}

#[allow(dead_code)]
pub mod eventstore {
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
        pub fn all(&self) -> Result<Vec<CloudEvent>> {
            let guard = self.evts.lock().unwrap();
            let matches = guard.iter().cloned().collect();
            Ok(matches)
        }

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
}
