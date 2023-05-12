use rspotify::{model, prelude::*};
use spt::eventsourcing::domain;
use spt::eventsourcing::eventstore::JSONEventStore;
use spt::eventsourcing::prelude::*;
use spt::login;
use spt::types;
use spt::Commands;
use std::env;
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
    let config = match config {
        Ok(config) => config,
        Err(err) => {
            eprintln!("{}", err);
            std::process::exit(1)
        }
    };

    // Authenticate with OAuth
    let spotify = match login::login() {
        Ok(spotify) => spotify,
        Err(why) => {
            eprintln!("Login failed: {why}");
            std::process::exit(1)
        }
    };

    // Load Users
    let before = Instant::now();
    let users = match spt::load_users(USER_FILE) {
        Ok(users) => {
            println!(
                "Loaded {} users from {} in {:.2?}",
                users.len(),
                USER_FILE,
                before.elapsed()
            );
            users
        }
        Err(err) => {
            eprintln!("Failed to load users from {}: {}", USER_FILE, err);
            std::process::exit(1)
        }
    };

    // Load stored events from file
    let before = Instant::now();
    let store_path = format!("{}/{}.json", DATA_DIR, DATA_FILE);
    let event_store = JSONEventStore::from_file(&store_path);
    let event_store = match event_store {
        Ok(store) => {
            println!(
                "Loaded {} events from {} in {:.2?}",
                store.len(),
                store_path,
                before.elapsed()
            );
            store
        }
        Err(err) => {
            eprintln!(
                "Failed to create eventstore from {}: {}\nUsing new one instead...",
                store_path, err
            );
            JSONEventStore::new()
        }
    };

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
        let user_playlists: Vec<model::SimplifiedPlaylist> = spotify
            .user_playlists(model::UserId::from_id_or_uri(&user.id).unwrap())
            .flatten()
            .collect();

        // filter out all playlists not owned by the user (e.g. the Daily Mix etc.)
        let user_playlists: Vec<model::SimplifiedPlaylist> = user_playlists
            .into_iter()
            .filter(|pl| pl.owner.id.to_string() == user.id)
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
            .map(|pl| spt::build_local(&pl.id.to_string(), &event_store).unwrap())
            .collect();
        println!(
            "Rebuilt {} playlists from memory in {:.2?}",
            localplaylists.len(),
            before.elapsed()
        );

        // Build playlists from spotify data
        let before = Instant::now();
        let playlists = user_playlists;
        println!(
            "Built {} playlists from spotify data in {:.2?}",
            playlists.len(),
            before.elapsed()
        );

        // Compare Playlists
        let before = Instant::now();
        let data = playlists.iter().zip(localplaylists.iter());
        for (playlist, local) in data {
            let plevent = spt::compare(&spotify, &local, &playlist, None, None).unwrap();
            if !plevent.is_empty() {
                // Calculate new state
                let _state = domain::PlaylistAggregate::apply_all(local.clone(), &plevent).unwrap();

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
    match event_store.save_events(&store_path) {
        Ok(_) => println!(
            "\nSaved all events to {} in {:.2?}",
            store_path,
            before.elapsed()
        ),
        Err(err) => eprintln!("\nFailed to save events to {}: {}", store_path, err),
    }

    println!("\nFinished in {:.2?}\n", runtime.elapsed());
}
