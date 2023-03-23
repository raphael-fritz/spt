mod login;

use rspotify::{
    model::{AdditionalType, Country, Market, PlayableItem},
    prelude::*,
};

use chrono::Utc;

fn main() {
    let spotify = login::login();

    let market = Some(Market::Country(Country::Austria));
    //let market: Option<Market> = None;
    //let additional_types = Some(&[AdditionalType::Episode]);
    //let additional_types: Option<&[AdditionalType; 1]> = None;
    let fields: Option<&str> = None;

    // Running the requests
    let user = spotify.current_user().expect("Couldn't get current user!");

    let user_playlists = spotify.user_playlists(user.id.clone());
    println!("User playlists:");
    for (i, playlist) in user_playlists.enumerate() {
        let playlist = playlist.unwrap();
        println!("{}: {:?}:", i, playlist.name,);
        for (i, track) in spotify
            .playlist_items(playlist.id, fields, market)
            .enumerate()
        {
            let track = track.unwrap().track.unwrap();
            match track {
                PlayableItem::Track(track) => {
                    println!(
                        "\t{}: {}-{}",
                        i,
                        track.artists.first().unwrap().name,
                        track.name
                    );
                }
                _ => (),
            }
        }
    }

    let currently_playing = spotify.current_playing(
        Some(Market::Country(Country::Austria)),
        Some(&[AdditionalType::Episode]),
    );
    let currently_playing = currently_playing.unwrap().unwrap().context;
    println!("Currently playing: {currently_playing:?}");

    let token_expiry = spotify
        .get_token()
        .lock()
        .unwrap()
        .clone()
        .unwrap()
        .expires_at
        .unwrap();
    let diff = token_expiry.time() - Utc::now().time();
    println!(
        "User ID: {}\nToken expires in: {}:{}:{}",
        user.id,
        (diff.num_seconds() / 60) / 60,
        (diff.num_seconds() / 60) % 60,
        diff.num_seconds() % 60
    );
}
