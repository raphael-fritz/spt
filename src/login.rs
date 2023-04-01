use rspotify::{prelude::*, scopes, AuthCodeSpotify, Credentials, OAuth};
use serde_json;
use std::error;
use std::fmt;
use std::fs;
use std::path::Path;

const TOKEN_PATH: &str = "token.tmp";
const CLIENT_ID: &str = "7d5c06725d314f0b975c88c7ca23b4d8";
const CLIENT_SECRET: &str = "832bd9a9d9144c62a3b1c3e9c26906ff";
const REDIRECT_URI: &str = "http://localhost:65432";
const SCOPES: &str = "playlist-modify-public, user-read-currently-playing";

#[derive(Debug)]
pub enum AuthenticationError {
    IOError(std::io::Error),
    ParseError(serde_json::Error),
    ClientError(rspotify::ClientError),
}

impl fmt::Display for AuthenticationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AuthenticationError::IOError(err) => write!(f, "{}", err),
            AuthenticationError::ParseError(err) => write!(f, "{}", err),
            AuthenticationError::ClientError(err) => write!(f, "{}", err),
        }
    }
}

impl From<std::io::Error> for AuthenticationError {
    fn from(err: std::io::Error) -> Self {
        AuthenticationError::IOError(err)
    }
}

impl From<serde_json::Error> for AuthenticationError {
    fn from(err: serde_json::Error) -> Self {
        AuthenticationError::ParseError(err)
    }
}

impl From<rspotify::ClientError> for AuthenticationError {
    fn from(err: rspotify::ClientError) -> Self {
        AuthenticationError::ClientError(err)
    }
}

impl error::Error for AuthenticationError {}

fn load_token(path: &Path) -> Result<rspotify::Token, AuthenticationError> {
    let token = fs::read_to_string(&path)?;
    let token = serde_json::from_str(&token)?;
    Ok(token)
}

fn save_token(token: rspotify::Token, path: &Path) -> Result<(), AuthenticationError> {
    let file = fs::File::create(&path)?;
    Ok(serde_json::to_writer(file, &token)?)
}

fn auth_with_prev_token(spotify: &AuthCodeSpotify) -> Result<(), AuthenticationError> {
    let prev_token = load_token(Path::new(TOKEN_PATH))?;
    *spotify.token.lock().unwrap() = Some(prev_token);
    Ok(spotify.refresh_token()?)
}

fn auth_with_fresh_token(spotify: &AuthCodeSpotify) -> Result<(), AuthenticationError> {
    let url = spotify.get_authorize_url(true)?;
    spotify.prompt_for_token(&url)?;
    Ok(())
}

pub fn login() -> Result<AuthCodeSpotify, AuthenticationError> {
    let creds = Credentials::new(CLIENT_ID, CLIENT_SECRET);
    let oauth = OAuth {
        redirect_uri: REDIRECT_URI.to_string(),
        scopes: scopes!(SCOPES),
        ..Default::default()
    };
    let spotify = AuthCodeSpotify::new(creds, oauth);

    if auth_with_prev_token(&spotify).is_ok() {
        println!("Successfully authenticated with saved token!");
    } else {
        auth_with_fresh_token(&spotify)?;
        println!("Successfully authenticated with fresh token!");
    }

    let token = spotify.get_token();
    match &*token.lock().unwrap() {
        Some(token) => save_token(token.clone(), Path::new(TOKEN_PATH))?,
        None => println!("Couldn't save token for further use!"),
    };

    return Ok(spotify);
}
