use indicatif::{MultiProgress, ProgressBar, ProgressDrawTarget, ProgressIterator, ProgressStyle};
use rspotify::{model, prelude::*, ClientResult};
use spt::eventsourcing::domain;
use spt::eventsourcing::eventstore::JSONEventStore;
use spt::eventsourcing::prelude::*;
use spt::login;
use spt::types;
use spt::Commands;
use std::time::Instant;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const DATA_DIR: &str = "data";
const DATA_FILE: &str = "events";
const USER_FILE: &str = "data/users.json";

const MAIN_STYLE: &str = "[{elapsed_precise}][{bar:40.green/white}][{pos:>3}/{len:3}]: {msg}";
const LOWER_STYLE: &str = "          [{bar:40.green/white}][{pos:>3}/{len:3}]: {msg}";
const PROGRESS_CHARS: &str = "=>-";

fn main() {
    println!("Spotify-Playlist-Tracker-v{}\n", VERSION);

    let config = spt::Commands::build();
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
                display_name: Some(config[0].clone()),
                id: rspotify::model::UserId::from_id(config[1].clone())
                    .unwrap()
                    .to_string(),
            };
            spt::add_users(USER_FILE, user).unwrap();
            vec![]
        }
    };

    let target = ProgressDrawTarget::stderr_with_hz(120);
    let multi = MultiProgress::with_draw_target(target);
    let stylemain = ProgressStyle::with_template(MAIN_STYLE)
        .unwrap()
        .progress_chars(PROGRESS_CHARS);
    let style = ProgressStyle::with_template(LOWER_STYLE)
        .unwrap()
        .progress_chars(PROGRESS_CHARS);
    let pb = ProgressBar::new(users.len() as u64).with_style(stylemain);
    let pb = multi.insert(0, pb);
    pb.tick();
    let mut pbs: Vec<ProgressBar> = Vec::new();

    for user in &users {
        if !pbs.is_empty() {
            for pb in &pbs {
                multi.remove(pb);
            }
            pbs.clear();
        }

        let nameorid = user.display_name.clone().unwrap_or(user.id.clone());
        pb.set_message(format!("{}", nameorid));
        pb.set_prefix(format!("{}", nameorid));

        // Build playlists from spotify data
        let user_playlists: Vec<ClientResult<model::SimplifiedPlaylist>> = spotify
            .user_playlists(model::UserId::from_id_or_uri(&user.id).unwrap())
            .collect();

        let pb1 = ProgressBar::new(user_playlists.len() as u64).with_style(style.clone());
        let pb1 = multi.insert(1, pb1);
        pb1.set_message("Building playlists from spotify data");
        pbs.push(pb1.clone());
        pb1.tick();
        let before = Instant::now();
        let user_playlists: Vec<model::SimplifiedPlaylist> = user_playlists
            .into_iter()
            .progress_with(pb1.clone())
            .flatten() // throw away Resullt:Err(_) entries
            .filter(|pl| pl.owner.id.to_string() == user.id) // filter out all playlists not owned by the user (e.g. the Daily Mix etc.)
            .collect();
        let playlists = &user_playlists;
        pb1.finish_with_message(format!(
            "Built playlists from spotify data in {:.2?}",
            before.elapsed()
        ));

        // Rebuild playlist state from events
        let pb2 = ProgressBar::new(user_playlists.len() as u64).with_style(style.clone());
        let pb2 = multi.insert(2, pb2);
        pb2.set_message("Rebuilding playlists from memory");
        pbs.push(pb2.clone());
        pb2.tick();
        let before = Instant::now();
        let localplaylists: Vec<domain::PlaylistData> = user_playlists
            .iter()
            .progress_with(pb2.clone())
            .map(|pl| spt::build_local(&pl.id.to_string(), &event_store).unwrap())
            .collect();
        pb2.finish_with_message(format!(
            "Rebuilt playlists from memory in {:.2?}",
            before.elapsed()
        ));

        // Compare Playlists
        let pb3 = ProgressBar::new(user_playlists.len() as u64).with_style(style.clone());
        let pb3 = multi.insert(3, pb3);
        pb3.set_message("Comparing playlists");
        pbs.push(pb3.clone());
        pb3.tick();
        let before = Instant::now();
        for (playlist, local) in playlists.iter().zip(localplaylists.iter()) {
            let plevent = spt::compare(
                &user.display_name.clone().unwrap_or(user.id.clone()),
                &multi,
                &spotify,
                &local,
                &playlist,
                None,
                None,
            )
            .unwrap();
            if !plevent.is_empty() {
                // Calculate new state
                let _state = domain::PlaylistAggregate::apply_all(local.clone(), &plevent).unwrap();

                // Save all events
                for event in plevent {
                    let _store_result = event_store.append(event.clone(), "playlists").unwrap();
                }
            }
            pb3.inc(1);
            pb3.tick();
        }
        pb3.finish_with_message(format!(
            "Compared and updated playlists in {:.2?}",
            before.elapsed()
        ));

        pb.inc(1);
    }
    pb.finish_with_message("Finished!");
    for pb in &pbs {
        multi.remove(pb);
    }
    pb.tick();

    // Write to disk
    let before = Instant::now();
    match event_store.save_events(&store_path) {
        Ok(_) => println!(
            "Saved all events to {} in {:.2?}",
            store_path,
            before.elapsed()
        ),
        Err(err) => eprintln!("Failed to save events to {}: {}", store_path, err),
    }
}
