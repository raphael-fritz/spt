mod eventsourcing;
mod login;
mod types;

use crate::eventsourcing::domain;
use crate::eventsourcing::eventstore::JSONEventStore;
use crate::eventsourcing::prelude::*;
use rspotify::{model::SimplifiedPlaylist, prelude::*, ClientResult};
use std::collections::HashSet;
use std::fmt::Write;

const DATA_DIR: &str = "data";

fn main() {
    // Authenticate with OAuth
    let spotify = match login::login() {
        Ok(spotify) => spotify,
        Err(why) => panic!("Login failed: {why}"),
    };

    // Fetch spotify data
    let user = spotify.current_user().unwrap();
    let user_playlists: Vec<ClientResult<SimplifiedPlaylist>> =
        spotify.user_playlists(user.id.clone()).collect();

    for playlist in user_playlists {
        let playlist = playlist.unwrap();

        // Load stored events from file
        let id = playlist.id.clone().to_string();
        let id = id.split(":").last().unwrap();
        let mut store_path = String::new();
        write!(store_path, "{}/{}.json", DATA_DIR, id).unwrap();
        let playlist_store = JSONEventStore::from_file(&store_path);
        let events = playlist_store.all().unwrap();

        // Rebuild playlist state from events
        let state = domain::PlaylistData::new();
        let events: Vec<domain::PlaylistEvent> = events
            .into_iter()
            .map(|evt| domain::PlaylistEvent::from(evt))
            .collect();
        let state = domain::PlaylistAggregate::apply_all(&state, &events).unwrap();
        if state != domain::PlaylistData::new() {
            println!(
                "Rebuilt playlist {} ( {} ) from memory",
                state.data.name, state.data.id
            );
        }

        // Build playlist from spotify data
        let playlist = types::Playlist::from_id(&spotify, playlist.id.clone(), None, None).unwrap();

        // compare local and new version and create events if changes occured
        let mut plevent: Vec<domain::PlaylistEvent> = Vec::new();
        if events.is_empty() {
            println!("Created {} ( {} )", playlist.name, playlist.id);
            let cmd = domain::PlaylistCommand::CreatePlaylist(playlist.clone());
            let evts = domain::PlaylistAggregate::handle_command(&state, &cmd).unwrap();
            plevent.extend(evts);
        } else if state.data != playlist {
            // UpdateName Event
            if state.data.name != playlist.name {
                println!("Updated name for {} ( {} )", state.data.name, state.data.id);
                let cmd = domain::PlaylistCommand::UpdateName(playlist.name.clone());
                let evts = domain::PlaylistAggregate::handle_command(&state, &cmd).unwrap();
                plevent.extend(evts);
            }

            // UpdateDescription Event
            if state.data.description != playlist.description {
                println!(
                    "Updated description for {} ( {} )",
                    state.data.name, state.data.id
                );
                let cmd = domain::PlaylistCommand::UpdateDesciption(playlist.description);
                let evts = domain::PlaylistAggregate::handle_command(&state, &cmd).unwrap();
                plevent.extend(evts);
            }

            if state.data.tracks != playlist.tracks {
                let playlist: HashSet<types::PlaylistItem> =
                    playlist.tracks.iter().cloned().collect();
                let localplaylist: HashSet<types::PlaylistItem> =
                    state.data.tracks.iter().cloned().collect();

                // AddTracks Event
                let addedtracks: HashSet<_> = playlist.difference(&localplaylist).collect();
                if !addedtracks.is_empty() {
                    println!("Added tracks to {} ( {} ) ", state.data.name, state.data.id);
                    let cmd = domain::PlaylistCommand::AddTracks(
                        addedtracks.into_iter().cloned().collect(),
                    );
                    let evts = domain::PlaylistAggregate::handle_command(&state, &cmd).unwrap();
                    plevent.extend(evts);
                }

                // RemovedTracks Event
                let removedtracks: HashSet<_> = localplaylist.difference(&playlist).collect();
                if !removedtracks.is_empty() {
                    println!(
                        "Removed tracks from {} ( {} ) ",
                        state.data.name, state.data.id
                    );
                    let cmd = domain::PlaylistCommand::RemoveTracks(
                        removedtracks.into_iter().cloned().collect(),
                    );
                    let evts = domain::PlaylistAggregate::handle_command(&state, &cmd).unwrap();
                    plevent.extend(evts);
                }
            }
        }

        if !plevent.is_empty() {
            println!(
                "Applied {} event to {} ( {} )",
                plevent.len(),
                playlist.name,
                playlist.id
            );

            // Calculate new state
            let _state = domain::PlaylistAggregate::apply_all(&state, &plevent).unwrap();

            // Save all events
            for event in plevent {
                let _store_result = playlist_store.append(event.clone(), "playlists").unwrap();
            }

            // Write to disk
            playlist_store.save_events(&store_path);
        } else {
            println!(
                "Playlist {} has not changed since last update!",
                playlist.name
            );
        }
    }
}
