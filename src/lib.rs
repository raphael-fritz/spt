pub mod eventsourcing;
pub mod login;
pub mod types;

use crate::eventsourcing::domain;
use crate::eventsourcing::eventstore::JSONEventStore;
use crate::eventsourcing::prelude::*;
use rspotify::model;
use rspotify::AuthCodeSpotify;
use std::collections::HashSet;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader};
use std::path::Path;

#[derive(Debug, Clone)]
pub enum Commands<'a> {
    DEFAULT,
    SINGLE,
    AddUser(Option<&'a [String]>),
}

impl Commands<'_> {
    pub fn build_local(args: &[String]) -> Result<Commands, &'static str> {
        if args.len() == 3 {
            Ok(Commands::AddUser(Some(&args[2..args.len()])))
        } else if args.len() == 2 {
            Ok(Commands::SINGLE)
        } else if args.len() == 1 {
            Ok(Commands::DEFAULT)
        } else {
            Err("USAGE: spt.exe -n {{name}} {{id}} to add a new name\n       spt.exe to update data")
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
    let state = domain::PlaylistAggregate::apply_all(&state, &events)?;
    Ok(state)
}

/// compare local and new version and return events if changes occured
pub fn compare(
    client: &AuthCodeSpotify,
    state: &domain::PlaylistData,
    playlist: &model::FullPlaylist,
    fields: Option<&str>,
    market: Option<rspotify::model::Market>,
) -> Result<Vec<domain::PlaylistEvent>, types::SPTError> {
    let mut plevents: Vec<domain::PlaylistEvent> = Vec::new();

    if state.generation == 0 {
        println!("Created {} ( {} )", playlist.name, playlist.id);
        let playlist = types::Playlist::from_id(&client, playlist.id.clone(), fields, market)?;
        let cmd = domain::PlaylistCommand::CreatePlaylist(playlist.id.clone(), playlist.clone());
        let evts = domain::PlaylistAggregate::handle_command(&state, &cmd)?;
        plevents.extend(evts);
    } else {
        // Saved my ass already, good assert
        assert!(state.data.id == playlist.id.to_string());

        // UpdateName Event
        if state.data.name != playlist.name {
            println!("Updated name for {} ( {} )", state.data.name, state.data.id);
            let cmd =
                domain::PlaylistCommand::UpdateName(playlist.id.to_string(), playlist.name.clone());
            let evts = domain::PlaylistAggregate::handle_command(&state, &cmd)?;
            plevents.extend(evts);
        }

        // UpdateDescription Event
        if state.data.description != playlist.description {
            println!(
                "Updated description for {} ( {} )",
                state.data.name, state.data.id
            );
            let cmd = domain::PlaylistCommand::UpdateDesciption(
                playlist.id.to_string(),
                playlist.description.clone(),
            );
            let evts = domain::PlaylistAggregate::handle_command(&state, &cmd)?;
            plevents.extend(evts);
        }

        if state.data.snapshot_id != playlist.snapshot_id {
            let playlist = types::Playlist::from_id(client, playlist.id.clone(), fields, market)?;
            if state.data.tracks != playlist.tracks {
                let plhash: HashSet<types::PlaylistItem> =
                    playlist.tracks.iter().cloned().collect();
                let localphash: HashSet<types::PlaylistItem> =
                    state.data.tracks.iter().cloned().collect();

                // AddTracks Event
                let addedtracks: HashSet<_> = plhash.difference(&localphash).collect();
                if !addedtracks.is_empty() {
                    println!("Added tracks to {} ( {} ) ", state.data.name, state.data.id);
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
                    println!(
                        "Removed tracks from {} ( {} ) ",
                        state.data.name, state.data.id
                    );
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
