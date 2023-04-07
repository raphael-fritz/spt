pub mod eventsourcing;
pub mod login;
pub mod types;

use crate::eventsourcing::domain;
use crate::eventsourcing::eventstore::JSONEventStore;
use crate::eventsourcing::prelude::*;
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
        if args.len() > 2 {
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
        let line = line.unwrap();
        let user: types::User = serde_json::from_str(&line).unwrap();
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
    let user = serde_json::to_string(&user).unwrap();
    write!(file, "{}\n", user).unwrap();
    Ok(())
}

/// Rebuild playlist state from events
pub fn build_local(origin_id: &String, pl_store: &JSONEventStore) -> domain::PlaylistData {
    let events: Vec<domain::PlaylistEvent> = pl_store.get_all(origin_id.to_string()).unwrap();
    let state = domain::PlaylistData::new();
    let state = domain::PlaylistAggregate::apply_all(&state, &events).unwrap();
    state
}

/// compare local and new version and return events if changes occured
pub fn compare(
    state: &domain::PlaylistData,
    playlist: &types::Playlist,
) -> Vec<domain::PlaylistEvent> {
    let mut plevents: Vec<domain::PlaylistEvent> = Vec::new();

    if state.generation == 0 {
        println!("Created {} ( {} )", playlist.name, playlist.id);
        let cmd = domain::PlaylistCommand::CreatePlaylist(playlist.id.clone(), playlist.clone());
        let evts = domain::PlaylistAggregate::handle_command(&state, &cmd).unwrap();
        plevents.extend(evts);
    } else if state.data != playlist.clone() {
        // Saved my ass already, good assert
        assert!(state.data.id == playlist.id);

        // UpdateName Event
        if state.data.name != playlist.name {
            println!("Updated name for {} ( {} )", state.data.name, state.data.id);
            let cmd =
                domain::PlaylistCommand::UpdateName(playlist.id.clone(), playlist.name.clone());
            let evts = domain::PlaylistAggregate::handle_command(&state, &cmd).unwrap();
            plevents.extend(evts);
        }

        // UpdateDescription Event
        if state.data.description != playlist.description {
            println!(
                "Updated description for {} ( {} )",
                state.data.name, state.data.id
            );
            let cmd = domain::PlaylistCommand::UpdateDesciption(
                playlist.id.clone(),
                playlist.description.clone(),
            );
            let evts = domain::PlaylistAggregate::handle_command(&state, &cmd).unwrap();
            plevents.extend(evts);
        }

        if state.data.tracks != playlist.tracks {
            let plhash: HashSet<types::PlaylistItem> = playlist.tracks.iter().cloned().collect();
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
                let evts = domain::PlaylistAggregate::handle_command(&state, &cmd).unwrap();
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
                let evts = domain::PlaylistAggregate::handle_command(&state, &cmd).unwrap();
                plevents.extend(evts);
            }
        }
    }

    return plevents;
}
