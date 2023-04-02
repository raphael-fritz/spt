use super::uevents::UniqueEvent;
use super::{prelude::*, Aggregate, Result};
use crate::types;
use serde::{Deserialize, Serialize};
use std::fmt;

const DOMAIN_VERSION: &str = "1.0";

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
}
impl From<UniqueEvent> for PlaylistEvent {
    fn from(evt: UniqueEvent) -> Self {
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
