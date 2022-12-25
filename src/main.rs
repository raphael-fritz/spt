use rspotify::{
    //model::{AdditionalType, Country, Market},
    prelude::*,
    scopes,
    AuthCodeSpotify,
    Config,
    Credentials,
    OAuth,
};

use serde_json;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

fn load_token() {
    let path = Path::new("hello.txt");
    let display = path.display();

    let mut file = match File::open(&path) {
        Err(why) => panic!("Couldn't open {display}: {why}"),
        Ok(file) => file,
    };

    let mut s = String::new();
    match file.read_to_string(&mut s) {
        Err(why) => panic!("couldn't read {display}: {why}"),
        Ok(_) => print!("{display} contains:\n{s}"),
    }
}

fn save_token(token: rspotify::Token) {
    let path = Path::new("token.tmp");
    let display = path.display();

    // Open a file in write-only mode, returns `io::Result<File>`
    let mut file = match File::create(&path) {
        Err(why) => panic!("couldn't create {}: {}", display, why),
        Ok(file) => file,
    };

    let t = serde_json::to_string(&token).unwrap();
    // Write the `LOREM_IPSUM` string to `file`, returns `io::Result<()>`
    match file.write_all(t.as_bytes()) {
        Err(why) => panic!("couldn't write to {}: {}", display, why),
        Ok(_) => println!("successfully wrote to {}", display),
    }
}

#[tokio::main]
async fn main() {
    let creds = Credentials::new(
        "7d5c06725d314f0b975c88c7ca23b4d8",
        "832bd9a9d9144c62a3b1c3e9c26906ff",
    );

    let oauth = OAuth {
        redirect_uri: "http://localhost:65432".to_string(),
        scopes: scopes!("playlist-modify-public, user-read-recently-played"),
        ..Default::default()
    };

    let spotify = AuthCodeSpotify::new(creds, oauth);

    // Obtaining the access token
    let url = spotify.get_authorize_url(false).unwrap();

    // This function requires the `cli` feature enabled.
    spotify
        .prompt_for_token(&url)
        .await
        .expect("Couldn't authenticate succesfully!");

    let token = spotify.get_token().lock().await.unwrap().clone().unwrap();
    save_token(token.clone());

    // Running the requests
    let user = spotify
        .current_user()
        .await
        .expect("Couldn't get current user!");

    //println!("Response: {user:?}");
    println!(
        "User ID: {0}\nToken expires in: {1}",
        user.id,
        token.expires_at.unwrap().to_string()
    );
    println!("Token: {token:?}");
}
