pub mod eventsourcing;
pub mod login;
pub mod types;

use crate::eventsourcing::domain;
use crate::eventsourcing::eventstore::JSONEventStore;
use crate::eventsourcing::prelude::*;
use rspotify::model;
use rspotify::AuthCodeSpotify;
use std::collections::HashSet;
use std::env;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader};
use std::path::Path;

#[derive(Debug, Clone)]
pub enum Commands {
    DEFAULT,
    SINGLE,
    AddUser(Vec<String>),
}
impl Commands {
    pub fn build() -> Result<Commands, &'static str> {
        let args: Vec<String> = env::args().collect();
        let len = args.len();
        if len <= 1 {
            Ok(Commands::DEFAULT)
        } else {
            match (len, args[1].as_str()) {
                (2, "-s") => Ok(Commands::SINGLE),
                (4, "-n") => Ok(Commands::AddUser(args[2..args.len()].to_vec())),
                _ => Err("USAGE: spt.exe to update data\n       \
                                 spt.exe -n {{name}} {{id}} to add a new name\n       \
                                 spt.exe -s to update data for only the first user"),
            }
        }
    }
}

pub fn load_users<P: AsRef<Path> + ?Sized + std::convert::AsRef<std::ffi::OsStr>>(
    path: &P,
) -> Result<Vec<types::User>, std::io::Error> {
    let file = File::open(path)?;
    let lines = BufReader::new(file).lines();
    let mut users = Vec::<types::User>::new();

    for line in lines {
        let line = line?;
        let user: types::User = serde_json::from_str(&line)?;
        users.push(user);
    }

    Ok(users)
}

pub fn add_users<P: AsRef<Path> + ?Sized + std::convert::AsRef<std::ffi::OsStr>>(
    path: &P,
    user: types::User,
) -> Result<(), std::io::Error> {
    use std::io::Write;

    let mut file = OpenOptions::new()
        .write(true)
        .append(true)
        .create(true)
        .open(path)?;
    let user = serde_json::to_string(&user)?;
    write!(file, "{}\n", user)?;
    Ok(())
}

/// Rebuild playlist state from events
pub fn build_local(
    origin_id: &String,
    pl_store: &JSONEventStore,
) -> eventsourcing::Result<domain::PlaylistData> {
    let events: Vec<domain::PlaylistEvent> = pl_store.get_all(origin_id.to_string());
    let state = domain::PlaylistData::new();
    let state = domain::PlaylistAggregate::apply_all(state, &events)?;
    Ok(state)
}

/// compare local and new version and return events if changes occured
pub fn compare(
    username: &str,
    multi: &indicatif::MultiProgress,
    client: &AuthCodeSpotify,
    state: &domain::PlaylistData,
    playlist: &model::SimplifiedPlaylist,
    fields: Option<&str>,
    market: Option<rspotify::model::Market>,
) -> Result<Vec<domain::PlaylistEvent>, types::SPTError> {
    let mut plevents: Vec<domain::PlaylistEvent> = Vec::new();

    if state.generation == 0 {
        multi
            .println(format!(
                "[{}] Created {} ( {} )",
                username, playlist.name, playlist.id
            ))
            .unwrap();
        let playlist = types::Playlist::from_id(&client, playlist.id.clone(), fields, market)?;
        let cmd = domain::PlaylistCommand::CreatePlaylist(playlist.id.clone(), playlist.clone());
        let evts = domain::PlaylistAggregate::handle_command(&state, &cmd)?;
        plevents.extend(evts);
    } else {
        // Saved my ass already, good assert
        assert!(state.data.id == playlist.id.to_string());

        // UpdateName Event
        if state.data.name != playlist.name {
            multi
                .println(format!(
                    "[{}] Updated name for {} ( {} )",
                    username, state.data.name, state.data.id
                ))
                .unwrap();
            let cmd =
                domain::PlaylistCommand::UpdateName(playlist.id.to_string(), playlist.name.clone());
            let evts = domain::PlaylistAggregate::handle_command(&state, &cmd)?;
            plevents.extend(evts);
        }

        if state.data.snapshot_id != playlist.snapshot_id {
            let playlist = types::Playlist::from_id(client, playlist.id.clone(), fields, market)?;

            // UpdateDescription Event
            if state.data.description != playlist.description {
                multi
                    .println(format!(
                        "[{}] Updated description for {} ( {} )",
                        username, state.data.name, state.data.id
                    ))
                    .unwrap();
                let cmd = domain::PlaylistCommand::UpdateDesciption(
                    playlist.id.to_string(),
                    playlist.description.clone(),
                );
                let evts = domain::PlaylistAggregate::handle_command(&state, &cmd)?;
                plevents.extend(evts);
            }

            if state.data.tracks != playlist.tracks {
                let plhash: HashSet<types::PlaylistItem> =
                    playlist.tracks.iter().cloned().collect();
                let localphash: HashSet<types::PlaylistItem> =
                    state.data.tracks.iter().cloned().collect();

                // AddTracks Event
                let addedtracks: HashSet<_> = plhash.difference(&localphash).collect();
                if !addedtracks.is_empty() {
                    multi
                        .println(format!(
                            "[{}] Added tracks to {} ( {} ) ",
                            username, state.data.name, state.data.id
                        ))
                        .unwrap();
                    let cmd = domain::PlaylistCommand::AddTracks(
                        playlist.id.clone(),
                        playlist.snapshot_id.clone(),
                        addedtracks.into_iter().cloned().collect(),
                    );
                    let evts = domain::PlaylistAggregate::handle_command(&state, &cmd)?;
                    plevents.extend(evts);
                }

                // RemovedTracks Event
                let removedtracks: HashSet<_> = localphash.difference(&plhash).collect();
                if !removedtracks.is_empty() {
                    multi
                        .println(format!(
                            "[{}] Removed tracks from {} ( {} ) ",
                            username, state.data.name, state.data.id
                        ))
                        .unwrap();
                    let cmd = domain::PlaylistCommand::RemoveTracks(
                        playlist.id.clone(),
                        playlist.snapshot_id.clone(),
                        removedtracks.into_iter().cloned().collect(),
                    );
                    let evts = domain::PlaylistAggregate::handle_command(&state, &cmd)?;
                    plevents.extend(evts);
                }
            }
        }
    }

    Ok(plevents)
}
