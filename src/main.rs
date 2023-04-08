use rspotify::{model, prelude::*, ClientResult};
use spt::eventsourcing::domain;
use spt::eventsourcing::eventstore::JSONEventStore;
use spt::eventsourcing::prelude::*;
use spt::login;
use spt::types;
use spt::Commands;
use std::env;
use std::fmt::Write;
use std::time::Instant;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const DATA_DIR: &str = "data";
const DATA_FILE: &str = "events";
const USER_FILE: &str = "data/users.json";

fn main() {
    let runtime = Instant::now();
    println!("Spotify-Playlist-Tracker-v{}\n", VERSION);

    let args: Vec<String> = env::args().collect();
    let config = spt::Commands::build_local(&args);
    let config = config.unwrap();

    // Authenticate with OAuth
    let spotify = match login::login() {
        Ok(spotify) => spotify,
        Err(why) => panic!("Login failed: {why}"),
    };

    // Load Users
    let before = Instant::now();
    let users = spt::load_users(USER_FILE).unwrap();
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

    let users: Vec<types::User> = match config {
        Commands::SINGLE => users[0..1].to_vec(),
        Commands::DEFAULT => users,
        Commands::AddUser(config) => {
            let user = types::User {
                display_name: Some(config.unwrap()[0].clone()),
                id: rspotify::model::UserId::from_id(config.unwrap()[1].clone())
                    .unwrap()
                    .to_string(),
            };
            spt::add_users(USER_FILE, user).unwrap();
            vec![]
        }
    };

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
            .map(|pl| spt::build_local(&pl.id.to_string(), &event_store))
            .collect();
        println!(
            "Rebuilt {} playlists from memory in {:.2?}",
            localplaylists.len(),
            before.elapsed()
        );

        // Build playlists from spotify data
        let before = Instant::now();
        let playlists: Vec<model::FullPlaylist> = user_playlists
            .iter()
            .map(|res| res.as_ref().unwrap())
            .map(|pl| spotify.playlist(pl.id.clone(), None, None).unwrap())
            .collect();
        println!(
            "Built {} playlists from spotify data in {:.2?}",
            playlists.len(),
            before.elapsed()
        );

        // Compare Playlists
        let before = Instant::now();
        let data = playlists.iter().zip(localplaylists.iter());
        for (playlist, local) in data {
            let plevent = spt::compare(&spotify, &local, &playlist, None, None);
            if !plevent.is_empty() {
                // Calculate new state
                let _state = domain::PlaylistAggregate::apply_all(&local, &plevent).unwrap();

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
