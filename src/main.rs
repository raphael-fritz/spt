mod eventsourcing;
mod login;
mod types;

use crate::eventsourcing::domain;
use crate::eventsourcing::eventstore::JSONEventStore;
use crate::eventsourcing::prelude::*;
use rspotify::{model::SimplifiedPlaylist, prelude::*, ClientResult};
use std::collections::HashSet;
use std::fmt::Write;
use std::time::Instant;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const DATA_DIR: &str = "data";

fn main() {
    let runtime = Instant::now();
    println!("Spotify-Playlist-Tracker-v{}\n", VERSION);

    // Authenticate with OAuth
    let spotify = match login::login() {
        Ok(spotify) => spotify,
        Err(why) => panic!("Login failed: {why}"),
    };

    // Fetch spotify data
    let before = Instant::now();
    let user = spotify.current_user().unwrap();
    let user_playlists: Vec<ClientResult<SimplifiedPlaylist>> =
        spotify.user_playlists(user.id.clone()).collect();
    println!(
        "Fetched {} playlists from {} in {:.2?}",
        user_playlists.len(),
        user.display_name.unwrap(),
        before.elapsed()
    );

    // Load stored events from file
    let before = Instant::now();
    let id = user.id.clone().to_string();
    let id = id.split(":").last().unwrap();
    let mut store_path = String::new();
    write!(store_path, "{}/{}-{}.json", DATA_DIR, id, "playlists").unwrap();
    let playlist_store = JSONEventStore::from_file(&store_path);
    println!(
        "Loaded {} events from {} in {:.2?}",
        playlist_store.len(),
        store_path,
        before.elapsed()
    );

    let playlist_ids: Vec<rspotify::model::PlaylistId> = user_playlists
        .iter()
        .map(|res| res.as_ref().unwrap())
        .map(|pl| pl.id.clone())
        .collect();

    // Build playlists from spotify data
    let before = Instant::now();
    let playlists: Vec<types::Playlist> = playlist_ids
        .iter()
        .map(|p| types::Playlist::from_id(&spotify, p.clone(), None, None).unwrap())
        .collect();
    println!(
        "Built {} playlists from spotify data in {:.2?}",
        playlists.len(),
        before.elapsed()
    );

    // Rebuild playlist state from events
    let before = Instant::now();
    let localplaylists: Vec<domain::PlaylistData> = build(&playlist_ids, &playlist_store);
    println!(
        "Rebuilt {} playlists from memory in {:.2?}",
        localplaylists.len(),
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
                let _store_result = playlist_store.append(event.clone(), "playlists").unwrap();
            }
        }
    }
    println!(
        "Compared and updated {} playlists in {:.2?}",
        playlists.len(),
        before.elapsed()
    );

    // Write to disk
    let before = Instant::now();
    playlist_store.save_events(&store_path);
    println!(
        "Saved all events to {} in {:.2?}",
        store_path,
        before.elapsed()
    );

    println!("\nFinished in {:.2?}\n", runtime.elapsed());
}

/// Rebuild playlist state from events
fn build(
    pl_ids: &Vec<rspotify::model::PlaylistId>,
    pl_store: &JSONEventStore,
) -> Vec<domain::PlaylistData> {
    let mut localplaylists: Vec<domain::PlaylistData> = Vec::new();
    for id in pl_ids {
        let events: Vec<domain::PlaylistEvent> = pl_store.get_all(id.to_string()).unwrap();
        let state = domain::PlaylistData::new();
        let state = domain::PlaylistAggregate::apply_all(&state, &events).unwrap();
        localplaylists.push(state);
    }
    localplaylists
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
                    removedtracks.into_iter().cloned().collect(),
                );
                let evts = domain::PlaylistAggregate::handle_command(&state, &cmd).unwrap();
                plevents.extend(evts);
            }
        }
    }

    return plevents;
}
