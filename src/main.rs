mod domain;
mod eventstore;
mod login;
mod types;

use eventsourcing::{eventstore::EventStore, Aggregate};
use rspotify::{model::SimplifiedPlaylist, prelude::*, ClientResult};

const STORE_PATH: &str = "data.json";

fn main() {
    let spotify = match login::login() {
        Ok(spotify) => spotify,
        Err(why) => panic!("Login failed: {why}"),
    };

    let user = spotify.current_user().unwrap();

    let user_playlists: Vec<ClientResult<SimplifiedPlaylist>> =
        spotify.user_playlists(user.id.clone()).collect();
    let playlist = user_playlists.first().unwrap().as_ref().unwrap();
    let playlist = types::Playlist::from_id(spotify, playlist.id.clone(), None, None);

    let playlist_store = eventstore::JSONEventStore::from_file(STORE_PATH);
    let state = domain::PlaylistData {
        data: types::Playlist {
            collaborative: false,
            description: None,
            followers: 0,
            id: String::new(),
            name: String::new(),
            owner: types::User::new(),
            public: None,
            tracks: types::PlaylistItems(vec![]),
        },
        generation: 0,
    };
    println!("Initial State: {:#?}", state);

    let playlistcreation = domain::PlaylistCommand::CreatePlaylist(playlist.unwrap());
    let create_playlist =
        domain::PlaylistAggregate::handle_command(&state, &playlistcreation).unwrap();
    println!("Applied Event: {:#?}", create_playlist[0]);

    let state = domain::PlaylistAggregate::apply_all(&state, &create_playlist).unwrap();
    println!("State 1: {:#?}", state);

    let store_result = playlist_store
        .append(create_playlist[0].clone(), "playlists")
        .unwrap();
    println!("Store result: {:#?}", store_result);
    let builtevent = domain::PlaylistEvent::from(store_result);
    println!("Rebuilt event: {:#?}", builtevent);

    let namechange = domain::PlaylistCommand::UpdateName("lol".to_string());
    let change_name = domain::PlaylistAggregate::handle_command(&state, &namechange).unwrap();
    println!("Applied Event: {:#?}", change_name[0]);

    let state = domain::PlaylistAggregate::apply_all(&state, &change_name).unwrap();
    println!("State 2: {:#?}", state);

    let store_result = playlist_store
        .append(change_name[0].clone(), "playlists")
        .unwrap();
    println!("Store result: {:#?}", store_result);
    let builtevent = domain::PlaylistEvent::from(store_result);
    println!("Rebuilt event: {:#?}", builtevent);

    let eventstr = domain::PlaylistEvent::UpdatedName(String::new()).to_string();
    println!(
        "all {} events: {:#?}",
        eventstr,
        playlist_store.get_all(eventstr.as_str())
    );

    let eventstr = domain::PlaylistEvent::CreatedPlaylist(types::Playlist::new()).to_string();
    println!(
        "all {} events: {:#?}",
        eventstr,
        playlist_store.get_all(eventstr.as_str())
    );

    playlist_store.save_events(STORE_PATH);
}
