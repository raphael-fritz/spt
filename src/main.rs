use rspotify::{
    model::{AdditionalType, Country, Market},
    prelude::*,
    scopes, AuthCodeSpotify, Credentials, OAuth,
};

use chrono::Utc;
use serde_json;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

static TOKEN_PATH: &str = "token.tmp";

fn load_token(path: &Path) -> std::io::Result<rspotify::Token> {
    let display = path.display();

    let mut f = File::open(&path)?;
    let mut s = String::new();
    f.read_to_string(&mut s)?;

    let token: rspotify::Token = match serde_json::from_str(&s) {
        Err(why) => panic!("Couldn't load token from {display}: {why}"),
        Ok(token) => token,
    };
    return Ok(token);
}

fn save_token(token: rspotify::Token, path: &Path) {
    let display = path.display();

    // Open a file in write-only mode, returns `io::Result<File>`
    let mut file = match File::create(&path) {
        Err(why) => panic!("couldn't create {}: {}", display, why),
        Ok(file) => file,
    };

    let t = serde_json::to_string(&token).unwrap();
    match file.write_all(t.as_bytes()) {
        Err(why) => panic!("couldn't write to {}: {}", display, why),
        Ok(_) => println!("successfully wrote to {}", display),
    }
}

async fn auth_with_prev_token(spotify: &AuthCodeSpotify) {
    let prev_token = load_token(Path::new(TOKEN_PATH));
    *spotify.token.lock().await.unwrap() = Some(prev_token.unwrap());
    spotify
        .refresh_token()
        .await
        .expect("Couldn't refresh token!");
}

async fn auth_with_fresh_token(spotify: &AuthCodeSpotify, url: &str) {
    spotify
        .prompt_for_token(url)
        .await
        .expect("Couldn't authenticate succesfully!");

    let token = spotify.get_token().lock().await.unwrap().clone().unwrap();
    save_token(token.clone(), Path::new(TOKEN_PATH));
}

#[tokio::main]
async fn main() {
    let creds = Credentials::new(
        "7d5c06725d314f0b975c88c7ca23b4d8",
        "832bd9a9d9144c62a3b1c3e9c26906ff",
    );

    let oauth = OAuth {
        redirect_uri: "http://localhost:65432".to_string(),
        scopes: scopes!("playlist-modify-public, user-read-currently-playing"),
        ..Default::default()
    };

    let spotify = AuthCodeSpotify::new(creds, oauth);
    let url = spotify.get_authorize_url(false).unwrap();
    //auth_with_fresh_token(&spotify, &url).await;
    auth_with_prev_token(&spotify).await;

    // Running the requests
    let user = spotify
        .current_user()
        .await
        .expect("Couldn't get current user!");

    let currently_playing = spotify
        .current_playing(
            Some(Market::Country(Country::Austria)),
            Some(&[AdditionalType::Episode]),
        )
        .await;
    println!("Currently playing: {currently_playing:?}");

    let token_expiry = spotify
        .get_token()
        .lock()
        .await
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
