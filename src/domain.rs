use std::error::Error;

use eventsourcing::{eventstore::MemoryEventStore, prelude::*, Result};
use eventsourcing_derive::Event;
use rspotify::model::{PlaylistId, PlaylistItem, PublicUser};
use serde::{Deserialize, Serialize};

const DOMAIN_VERSION: &str = "1.0";

#[derive(Serialize, Deserialize, Debug, Clone, Event)]
#[event_type_version(DOMAIN_VERSION)]
enum PlaylistEvent {
    ChangedDescription(PlaylistId, Option<String>),
    ChangedName(PlaylistId, String),
    AddedTracks(PlaylistId, Vec<PlaylistItem>),
    RemovedTracks(PlaylistId, Vec<PlaylistItem>),
}

enum PlaylistCommand {
    ChangeDescription(PlaylistId, Option<String>),
    ChangeName(PlaylistId, String),
    AddTracks(PlaylistId, Vec<PlaylistItem>),
    RemoveTracks(PlaylistId, Vec<PlaylistItem>),
}

#[derive(Debug, Clone)]
struct PlaylistData {
    description: Option<String>,
    id: PlaylistId<'static>,
    name: String,
    owner: PublicUser,
    tracks: Vec<PlaylistItem>,
    generation: u64,
}

impl AggregateState for PlaylistData {
    fn generation(&self) -> u64 {
        self.generation
    }
}

struct Playlist;

impl Aggregate for Playlist {
    type Event = PlaylistEvent;
    type State = PlaylistData;
    type Command = PlaylistCommand;

    fn apply_event(state: &Self::State, evt: &Self::Event) -> Result<Self::State> {
        let state = match *evt {
            PlaylistEvent::ChangedName(_, newname) => PlaylistData {
                description: state.description,
                id: state.id,
                name: newname,
                owner: state.owner,
                tracks: state.tracks,
                generation: state.generation + 1,
            },
            _ => todo!(),
        };
        Ok(state)
    }

    fn handle_command(_state: &Self::State, cmd: &Self::Command) -> Result<Vec<Self::Event>> {
        // SHOULD DO: validate state and command

        // if validation passes...
        let evts = match cmd {
            PlaylistCommand::ChangeName(id, newname) => {
                vec![PlaylistEvent::ChangedName(id.clone(), newname.clone())]
            }
            _ => todo!(),
        };
        Ok(evts)
    }
}

// fn main() {
//     let _account_store = MemoryEventStore::new();
//
//     let deposit = BankCommand::DepositFunds("SAVINGS100".to_string(), 500);
//
//     let initial_state = AccountData {
//         balance: 800,
//         acctnum: "SAVINGS100".to_string(),
//         generation: 1,
//     };
//
//     let post_deposit = Account::handle_command(&initial_state, &deposit).unwrap();
//     let state = Account::apply_event(&initial_state, &post_deposit[0]).unwrap();
//
//     println!("{:#?}", post_deposit);
//     println!("{:#?}", state);
// }
