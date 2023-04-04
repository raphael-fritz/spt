mod eventsourcing;
mod login;
mod types;

use crate::eventsourcing::domain;
use crate::eventsourcing::eventstore::JSONEventStore;
use crate::eventsourcing::prelude::*;
use rspotify::{model, prelude::*, ClientResult};
use std::collections::HashSet;
use std::env;
use std::fmt::Write;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::time::Instant;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const DATA_DIR: &str = "data";
const DATA_FILE: &str = "events";
const USER_FILE: &str = "data/users.json";

#[derive(Debug, Clone)]
enum Commands<'a> {
    DEFAULT,
    SINGLE,
    AddUser(Option<&'a [String]>),
}

impl Commands<'_> {
    fn build_local(args: &[String]) -> Result<Commands, &'static str> {
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

fn main() {
    let runtime = Instant::now();
    println!("Spotify-Playlist-Tracker-v{}\n", VERSION);

    let args: Vec<String> = env::args().collect();
    let config = Commands::build_local(&args);
    let config = config.unwrap();

    // Authenticate with OAuth
    let spotify = match login::login() {
        Ok(spotify) => spotify,
        Err(why) => panic!("Login failed: {why}"),
    };

    // Load Users
    let before = Instant::now();
    let users = load_users(USER_FILE).unwrap();
    println!(
        "Loaded {} user from {} in {:.2?}",
        users.len(),
        USER_FILE,
        before.elapsed()
    );

    // Load stored events from file
    let before = Instant::now();
    let mut store_path = String::new();
    write!(store_path, "{}/{}.json", DATA_DIR, DATA_FILE).unwrap();
    let event_store = JSONEventStore::from_file(&store_path);
    println!(
        "Loaded {} events from {} in {:.2?}",
        event_store.len(),
        store_path,
        before.elapsed()
    );

    match config {
        Commands::AddUser(config) => {
            let user = types::User {
                display_name: Some(config.unwrap()[0].clone()),
                id: rspotify::model::UserId::from_id(config.unwrap()[1].clone())
                    .unwrap()
                    .to_string(),
            };
            add_users(USER_FILE, user).unwrap()
        }
        Commands::SINGLE => {
            let user = users[0].clone();

            // Fetch user playlists
            println!("Fetching data for {:?} ( {} )", user.display_name, user.id);
            let before = Instant::now();
            let user_playlists: Vec<ClientResult<model::SimplifiedPlaylist>> = spotify
                .user_playlists(model::UserId::from_id_or_uri(&user.id).unwrap())
                .collect();
            println!(
                "Fetched {} playlists from {:?} ( {} ) in {:.2?}",
                user_playlists.len(),
                user.display_name,
                user.id.to_string(),
                before.elapsed()
            );

            // Rebuild playlist state from events
            let before = Instant::now();
            let localplaylists: Vec<domain::PlaylistData> = user_playlists
                .iter()
                .map(|res| res.as_ref().unwrap())
                .map(|pl| build_local(&pl.id.to_string(), &event_store))
                .collect();
            println!(
                "Rebuilt {} playlists from memory in {:.2?}",
                localplaylists.len(),
                before.elapsed()
            );

            // Build playlists from spotify data
            let before = Instant::now();
            let playlists: Vec<types::Playlist> = user_playlists
                .iter()
                .map(|res| res.as_ref().unwrap())
                .map(|pl| pl.id.clone())
                .map(|p| types::Playlist::from_id(&spotify, p.clone(), None, None).unwrap())
                .collect();
            println!(
                "Built {} playlists from spotify data in {:.2?}",
                playlists.len(),
                before.elapsed()
            );

            let before = Instant::now();
            for (state, playlist) in localplaylists.iter().zip(playlists.iter()) {
                let plevent = compare(state, playlist);
                if !plevent.is_empty() {
                    // Calculate new state
                    let _state = domain::PlaylistAggregate::apply_all(&state, &plevent).unwrap();

                    // Save all events
                    for event in plevent {
                        let _store_result = event_store.append(event.clone(), "playlists").unwrap();
                    }
                }
            }
            println!(
                "Compared and updated {} playlists in {:.2?}",
                playlists.len(),
                before.elapsed()
            );

            println!("\nFinished in {:.2?}\n", runtime.elapsed());
        }
        _ => {
            // Fetch user data
            let users = load_users(USER_FILE).unwrap();
            let before = Instant::now();
            println!(
                "Loaded {} user from {} in {:.2?}",
                users.len(),
                USER_FILE,
                before.elapsed()
            );
            for user in users {
                // Fetch user playlists
                println!(
                    "\nFetching data for {:?} ( {} )",
                    user.display_name, user.id
                );
                let before = Instant::now();
                let user_playlists: Vec<ClientResult<model::SimplifiedPlaylist>> = spotify
                    .user_playlists(model::UserId::from_id_or_uri(&user.id).unwrap())
                    .collect();
                println!(
                    "Fetched {} playlists from {:?} ( {} ) in {:.2?}",
                    user_playlists.len(),
                    user.display_name,
                    user.id.to_string(),
                    before.elapsed()
                );

                // Rebuild playlist state from events
                let before = Instant::now();
                let localplaylists: Vec<domain::PlaylistData> = user_playlists
                    .iter()
                    .map(|res| res.as_ref().unwrap())
                    .map(|pl| build_local(&pl.id.to_string(), &event_store))
                    .collect();
                println!(
                    "Rebuilt {} playlists from memory in {:.2?}",
                    localplaylists.len(),
                    before.elapsed()
                );

                // Build playlists from spotify data
                let before = Instant::now();
                let playlists: Vec<types::Playlist> = user_playlists
                    .iter()
                    .map(|res| res.as_ref().unwrap())
                    .map(|pl| pl.id.clone())
                    .map(|p| types::Playlist::from_id(&spotify, p.clone(), None, None).unwrap())
                    .collect();
                println!(
                    "Built {} playlists from spotify data in {:.2?}",
                    playlists.len(),
                    before.elapsed()
                );

                let before = Instant::now();
                for (state, playlist) in localplaylists.iter().zip(playlists.iter()) {
                    let plevent = compare(state, playlist);
                    if !plevent.is_empty() {
                        // Calculate new state
                        let _state =
                            domain::PlaylistAggregate::apply_all(&state, &plevent).unwrap();

                        // Save all events
                        for event in plevent {
                            let _store_result =
                                event_store.append(event.clone(), "playlists").unwrap();
                        }
                    }
                }
                println!(
                    "Compared and updated {} playlists in {:.2?}",
                    playlists.len(),
                    before.elapsed()
                );
            }
        }
    }

    // Write to disk
    let before = Instant::now();
    event_store.save_events(&store_path);
    println!(
        "Saved all events to {} in {:.2?}",
        store_path,
        before.elapsed()
    );
    println!("\nFinished in {:.2?}\n", runtime.elapsed());
}

fn load_users<P: AsRef<Path> + ?Sized + std::convert::AsRef<std::ffi::OsStr>>(
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

fn add_users<P: AsRef<Path> + ?Sized + std::convert::AsRef<std::ffi::OsStr>>(
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
fn build_local(origin_id: &String, pl_store: &JSONEventStore) -> domain::PlaylistData {
    let events: Vec<domain::PlaylistEvent> = pl_store.get_all(origin_id.to_string()).unwrap();
    let state = domain::PlaylistData::new();
    let state = domain::PlaylistAggregate::apply_all(&state, &events).unwrap();
    state
}

/// compare local and new version and return events if changes occured
fn compare(state: &domain::PlaylistData, playlist: &types::Playlist) -> Vec<domain::PlaylistEvent> {
    let mut plevents: Vec<domain::PlaylistEvent> = Vec::new();

    if state.generation == 0 {
        println!("Created {} ( {} )", playlist.name, playlist.id);
        let cmd = domain::PlaylistCommand::CreatePlaylist(playlist.id.clone(), playlist.clone());
        let evts = domain::PlaylistAggregate::handle_command(&state, &cmd).unwrap();
        plevents.extend(evts);
    } else if state.data != playlist.clone() {
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
