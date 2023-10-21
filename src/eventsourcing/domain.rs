use super::uevents::UniqueEvent;
use super::{prelude::*, Aggregate, Dispatcher, Error, Kind, Result};
use crate::types;
use serde::{Deserialize, Serialize};

const DOMAIN_VERSION: &str = "1.0";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum PlaylistEvent {
    CreatedPlaylist(String, types::Playlist),
    UpdatedDesciption(String, Option<String>),
    UpdatedName(String, String),
    RemovedTracks(String, String, types::PlaylistItems),
    AddedTracks(String, String, types::PlaylistItems),
    DeletedPlaylist(String),
}
impl Event for PlaylistEvent {
    fn event_type_version(&self) -> &str {
        DOMAIN_VERSION
    }
    fn event_origin_id(&self) -> String {
        match self {
            PlaylistEvent::CreatedPlaylist(id, _) => id.clone(),
            PlaylistEvent::UpdatedDesciption(id, _) => id.clone(),
            PlaylistEvent::UpdatedName(id, _) => id.clone(),
            PlaylistEvent::AddedTracks(id, _, _) => id.clone(),
            PlaylistEvent::RemovedTracks(id, _, _) => id.clone(),
            PlaylistEvent::DeletedPlaylist(id) => id.clone(),
        }
    }
}
impl From<UniqueEvent> for PlaylistEvent {
    /// This will panic when the Event can't be converted and should only be used internally where it can be guaranteed that the data is valid
    /// Convert to a `Result<PlaylistEvent, serde_json::Error>` instead
    fn from(evt: UniqueEvent) -> Self {
        serde_json::from_value(evt.data).expect("UniqueEvent is not parseable to an PlaylistEvent")
    }
}
impl From<UniqueEvent> for std::result::Result<PlaylistEvent, serde_json::Error> {
    fn from(evt: UniqueEvent) -> Self {
        Ok(serde_json::from_value(evt.data)?)
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum PlaylistCommand {
    CreatePlaylist(String, types::Playlist),
    UpdateDesciption(String, Option<String>),
    UpdateName(String, String),
    AddTracks(String, String, types::PlaylistItems),
    RemoveTracks(String, String, types::PlaylistItems),
    DeletePlaylist(String),
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

    fn apply_event(state: Self::State, evt: &Self::Event) -> Result<Self::State> {
        let state = match &*evt {
            PlaylistEvent::CreatedPlaylist(_id, playlist) => PlaylistData {
                data: playlist.to_owned(),
                generation: state.generation + 1,
            },
            PlaylistEvent::UpdatedName(_id, newname) => PlaylistData {
                data: types::Playlist {
                    collaborative: state.data.collaborative,
                    followers: state.data.followers,
                    public: state.data.public,
                    description: state.data.description,
                    id: state.data.id,
                    name: newname.to_owned(),
                    owner: state.data.owner.clone(),
                    tracks: state.data.tracks.clone(),
                    snapshot_id: state.data.snapshot_id.clone(),
                },
                generation: state.generation + 1,
            },
            PlaylistEvent::UpdatedDesciption(_id, newdes) => PlaylistData {
                data: types::Playlist {
                    collaborative: state.data.collaborative,
                    followers: state.data.followers,
                    public: state.data.public,
                    description: newdes.to_owned(),
                    id: state.data.id.clone(),
                    name: state.data.name.clone(),
                    owner: state.data.owner.clone(),
                    tracks: state.data.tracks.clone(),
                    snapshot_id: state.data.snapshot_id.clone(),
                },
                generation: state.generation + 1,
            },
            PlaylistEvent::AddedTracks(_id, snapshot, tracks) => {
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
                        snapshot_id: snapshot.clone(),
                    },
                    generation: state.generation + 1,
                }
            }
            PlaylistEvent::RemovedTracks(_id, snapshot, tracks) => {
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
                        snapshot_id: snapshot.clone(),
                    },
                    generation: state.generation + 1,
                }
            }
            PlaylistEvent::DeletedPlaylist(_) => todo!(),
        };
        Ok(state)
    }
    fn handle_command(state: &Self::State, cmd: &Self::Command) -> Result<Vec<Self::Event>> {
        // SHOULD DO: validate state and command

        // Check that command id matches state id
        // This doesn't apply for the CreatedPlaylist variant
        if let PlaylistCommand::AddTracks(id, _, _)
        | PlaylistCommand::DeletePlaylist(id)
        | PlaylistCommand::RemoveTracks(id, _, _)
        | PlaylistCommand::UpdateDesciption(id, _)
        | PlaylistCommand::UpdateName(id, _) = cmd
        {
            if id.clone() != state.data.id {
                return Err(Error {
                    kind: Kind::CommandFailure("Mismatched id!".to_string()),
                });
            }
        };

        let evts = match cmd {
            PlaylistCommand::CreatePlaylist(id, playlist) => {
                vec![PlaylistEvent::CreatedPlaylist(
                    id.to_owned(),
                    playlist.to_owned(),
                )]
            }
            PlaylistCommand::UpdateName(id, newname) => {
                vec![PlaylistEvent::UpdatedName(
                    id.to_owned(),
                    newname.to_owned(),
                )]
            }
            PlaylistCommand::UpdateDesciption(id, newdes) => {
                vec![PlaylistEvent::UpdatedDesciption(
                    id.to_owned(),
                    newdes.to_owned(),
                )]
            }
            PlaylistCommand::AddTracks(id, snapshot_id, tracks) => {
                vec![PlaylistEvent::AddedTracks(
                    id.to_owned(),
                    snapshot_id.to_owned(),
                    tracks.to_owned(),
                )]
            }
            PlaylistCommand::RemoveTracks(id, snapshot_id, tracks) => {
                vec![PlaylistEvent::RemovedTracks(
                    id.to_owned(),
                    snapshot_id.to_owned(),
                    tracks.to_owned(),
                )]
            }
            PlaylistCommand::DeletePlaylist(id) => {
                vec![PlaylistEvent::DeletedPlaylist(id.to_owned())]
            }
        };
        Ok(evts)
    }
}

pub struct PlaylistDispatcher;
impl Dispatcher for PlaylistDispatcher {
    type Event = PlaylistEvent;
    type State = PlaylistData;
    type Command = PlaylistCommand;
    type Aggregate = PlaylistAggregate;

    fn dispatch(
        state: &Self::State,
        cmd: &Self::Command,
        store: &impl EventStore,
        stream: &str,
    ) -> Vec<Result<UniqueEvent>> {
        match Self::Aggregate::handle_command(state, cmd) {
            Ok(evts) => evts
                .into_iter()
                .map(|evt| store.append(evt, stream))
                .collect(),
            Err(e) => vec![Err(e)],
        }
    }
}
