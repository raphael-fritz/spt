mod domain;
mod login;
mod types;

use eventsourcing::eventstore::EventStore;
use eventsourcing::Aggregate;
use rspotify::{model::SimplifiedPlaylist, prelude::*, ClientResult};
use std::collections::HashSet;

const STORE_PATH: &str = "data.json";

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
    let playlist = user_playlists.first().unwrap().as_ref().unwrap();

    // Load stored events from file
    let playlist_store = domain::eventstore::JSONEventStore::from_file(STORE_PATH);
    let events = playlist_store.all().unwrap();

    // Rebuild playlist state from events
    let state = domain::PlaylistData::new();
    let events: Vec<domain::PlaylistEvent> = events
        .into_iter()
        .map(|evt| domain::PlaylistEvent::from(evt))
        .collect();
    let state = domain::PlaylistAggregate::apply_all(&state, &events).unwrap();
    println!("Rebuilt State: {:#?}", state);

    // Build playlist from spotify data
    let playlist = types::Playlist::from_id(spotify, playlist.id.clone(), None, None).unwrap();

    // compare local and new version and create events if changes occured
    let mut events: Vec<domain::PlaylistEvent> = Vec::new();
    if state.data != playlist {
        // UpdateName Event
        if state.data.name != playlist.name {
            let cmd = domain::PlaylistCommand::UpdateName(playlist.name.clone());
            let evts = domain::PlaylistAggregate::handle_command(&state, &cmd).unwrap();
            events.extend(evts);
        }

        // UpdateDescription Event
        if state.data.description != playlist.description {
            let cmd = domain::PlaylistCommand::UpdateDesciption(playlist.description);
            let evts = domain::PlaylistAggregate::handle_command(&state, &cmd).unwrap();
            events.extend(evts);
        }

        if state.data.tracks != playlist.tracks {
            let playlist: HashSet<types::PlaylistItem> = playlist.tracks.iter().cloned().collect();
            let localplaylist: HashSet<types::PlaylistItem> =
                state.data.tracks.iter().cloned().collect();

            // AddTracks Event
            let addedtracks: HashSet<_> = playlist.difference(&localplaylist).collect();
            if !addedtracks.is_empty() {
                let cmd =
                    domain::PlaylistCommand::AddTracks(addedtracks.into_iter().cloned().collect());
                let evts = domain::PlaylistAggregate::handle_command(&state, &cmd).unwrap();
                events.extend(evts);
            }

            // RemovedTracks Event
            let removedtracks: HashSet<_> = localplaylist.difference(&playlist).collect();
            if !removedtracks.is_empty() {
                let cmd = domain::PlaylistCommand::RemoveTracks(
                    removedtracks.into_iter().cloned().collect(),
                );
                let evts = domain::PlaylistAggregate::handle_command(&state, &cmd).unwrap();
                events.extend(evts);
            }
        }
    }

    if !events.is_empty() {
        println!("Applying events: {:#?}", events);

        // Calculate new state
        let _state = domain::PlaylistAggregate::apply_all(&state, &events).unwrap();

        // Save all events
        for event in events {
            let store_result = playlist_store.append(event.clone(), "playlists").unwrap();
            println!("Store result: {:#?}", store_result);
        }
    } else {
        println!(
            "Playlist {} has not changed since last update!",
            playlist.name
        );
    }

    // Write to disk
    playlist_store.save_events(STORE_PATH);
}
