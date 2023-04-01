pub mod login {
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
}

pub mod types {
    use chrono::prelude::{DateTime, Utc};
    use rspotify::{model, prelude::BaseClient, ClientResult};
    use serde::{Deserialize, Serialize};
    use std::ops;

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct User {
        pub display_name: Option<String>,
        pub id: String,
    }
    impl From<model::PublicUser> for User {
        fn from(item: model::PublicUser) -> Self {
            User {
                display_name: item.display_name,
                id: item.id.to_string(),
            }
        }
    }
    impl From<model::PrivateUser> for User {
        fn from(item: model::PrivateUser) -> Self {
            User {
                display_name: item.display_name,
                id: item.id.to_string(),
            }
        }
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct Artist {
        name: String,
        id: Option<String>,
    }
    impl From<model::SimplifiedArtist> for Artist {
        fn from(item: model::SimplifiedArtist) -> Self {
            Artist {
                name: item.name,
                id: item.id.map(|id| id.to_string()),
            }
        }
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct Album {
        artists: Vec<Artist>,
        id: Option<String>,
        name: String,
    }
    impl From<model::SimplifiedAlbum> for Album {
        fn from(item: model::SimplifiedAlbum) -> Self {
            Album {
                artists: item
                    .artists
                    .into_iter()
                    .map(|artist| Artist::from(artist))
                    .collect(),
                id: item.id.map(|id| id.to_string()),
                name: item.name,
            }
        }
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct Track {
        artists: Vec<Artist>,
        album: Album,
        id: Option<String>,
        name: String,
    }
    impl From<model::FullTrack> for Track {
        fn from(item: model::FullTrack) -> Self {
            Track {
                artists: item
                    .artists
                    .into_iter()
                    .map(|artist| Artist::from(artist))
                    .collect(),
                album: Album::from(item.album),
                id: item.id.map(|id| id.to_string()),
                name: item.name,
            }
        }
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct Episode {
        id: String,
        name: String,
    }
    impl From<model::FullEpisode> for Episode {
        fn from(item: model::FullEpisode) -> Self {
            Episode {
                id: item.id.to_string(),
                name: item.name,
            }
        }
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub enum PlayableItem {
        Track(Track),
        Episode(Episode),
    }
    impl From<model::PlayableItem> for PlayableItem {
        fn from(item: model::PlayableItem) -> Self {
            match item {
                model::PlayableItem::Track(track) => PlayableItem::Track(Track::from(track)),
                model::PlayableItem::Episode(epi) => PlayableItem::Episode(Episode::from(epi)),
            }
        }
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct PlaylistItem {
        added_at: Option<DateTime<Utc>>,
        added_by: Option<User>,
        track: Option<PlayableItem>,
    }
    impl From<model::PlaylistItem> for PlaylistItem {
        fn from(item: model::PlaylistItem) -> Self {
            PlaylistItem {
                added_at: item.added_at,
                added_by: item.added_by.map(|user| User::from(user)),
                track: match item.track {
                    Some(model::PlayableItem::Track(track)) => {
                        Some(PlayableItem::Track(Track::from(track)))
                    }
                    Some(model::PlayableItem::Episode(episode)) => {
                        Some(PlayableItem::Episode(Episode::from(episode)))
                    }
                    None => None,
                },
            }
        }
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct PlaylistItems(pub Vec<PlaylistItem>);
    impl ops::Deref for PlaylistItems {
        type Target = Vec<PlaylistItem>;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct Playlist {
        pub collaborative: bool,
        pub description: Option<String>,
        pub followers: u32,
        pub id: String,
        pub name: String,
        pub owner: User,
        pub public: Option<bool>,
        pub tracks: PlaylistItems,
    }
    impl From<model::FullPlaylist> for Playlist {
        fn from(item: model::FullPlaylist) -> Self {
            Playlist {
                collaborative: item.collaborative,
                description: item.description,
                followers: item.followers.total,
                id: item.id.to_string(),
                name: item.name,
                owner: User::from(item.owner),
                public: item.public,
                tracks: PlaylistItems(vec![]),
            }
        }
    }
    impl From<Vec<ClientResult<model::PlaylistItem>>> for PlaylistItems {
        fn from(items: Vec<ClientResult<model::PlaylistItem>>) -> Self {
            PlaylistItems(
                items
                    .into_iter()
                    .map(|res| res.unwrap())
                    .map(|item| PlaylistItem::from(item))
                    .collect(),
            )
        }
    }

    impl Playlist {
        pub fn new(
            client: rspotify::AuthCodeSpotify,
            playlist_id: model::PlaylistId,
            fields: Option<&str>,
            market: Option<rspotify::model::Market>,
        ) -> Playlist {
            let playlist = client.playlist(playlist_id, fields, market).unwrap();
            let tracks: Vec<ClientResult<rspotify::model::PlaylistItem>> = client
                .playlist_items(playlist.id.clone(), fields, market)
                .collect();
            let mut playlist = Playlist::from(playlist);
            playlist.tracks = PlaylistItems::from(tracks);
            return playlist;
        }
    }
}

pub mod domain {
    use crate::types;
    use eventsourcing::{prelude::*, Aggregate, Result};
    use serde::{Deserialize, Serialize};
    use std::fmt;

    const DOMAIN_VERSION: &str = "1.0";
    const EVENT_SOURCE: &str = "events://SPT";

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub enum PlaylistEvent {
        CreatedPlaylist(types::Playlist),
        UpdatedDesciption(Option<String>),
        UpdatedName(String),
        UpdatedTracks(Vec<types::Track>),
        DeletedPlaylist(),
    }
    impl fmt::Display for PlaylistEvent {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                PlaylistEvent::CreatedPlaylist(_) => write!(f, "PlaylistEvent.CreatedPlaylist"),
                PlaylistEvent::UpdatedDesciption(_) => write!(f, "PlaylistEvent.UpdatedDesciption"),
                PlaylistEvent::UpdatedName(_) => write!(f, "PlaylistEvent.UpdatedName"),
                PlaylistEvent::UpdatedTracks(_) => write!(f, "PlaylistEvent.UpdatedTracks"),
                PlaylistEvent::DeletedPlaylist() => write!(f, "PlaylistEvent.DeletedPlaylist"),
            }
        }
    }
    impl Event for PlaylistEvent {
        fn event_type_version(&self) -> &str {
            DOMAIN_VERSION
        }
        fn event_type(&self) -> &str {
            match self {
                PlaylistEvent::CreatedPlaylist(_) => "PlaylistEvent.CreatedPlaylist",
                PlaylistEvent::UpdatedDesciption(_) => "PlaylistEvent.UpdatedDesciption",
                PlaylistEvent::UpdatedName(_) => "PlaylistEvent.UpdatedName",
                PlaylistEvent::UpdatedTracks(_) => "PlaylistEvent.UpdatedTracks",
                PlaylistEvent::DeletedPlaylist() => "PlaylistEvent.DeletedPlaylist",
            }
        }
        fn event_source(&self) -> &str {
            EVENT_SOURCE
        }
    }

    #[allow(dead_code)]
    #[derive(Debug)]
    pub enum PlaylistCommand {
        CreatePlaylist(types::Playlist),
        UpdateDesciption(Option<String>),
        UpdateName(String),
        UpdateTracks(Vec<types::Track>),
        DeletePlaylist(),
    }

    #[derive(Debug, Clone)]
    pub struct PlaylistData {
        pub data: types::Playlist,
        pub generation: u64,
    }
    impl AggregateState for PlaylistData {
        fn generation(&self) -> u64 {
            self.generation
        }
    }
    pub struct PlaylistAggregate;
    impl Aggregate for PlaylistAggregate {
        type Event = PlaylistEvent;
        type State = PlaylistData;
        type Command = PlaylistCommand;

        fn apply_event(state: &Self::State, evt: &Self::Event) -> Result<Self::State> {
            let state = match &*evt {
                PlaylistEvent::CreatedPlaylist(playlist) => PlaylistData {
                    data: playlist.to_owned(),
                    generation: state.generation + 1,
                },
                PlaylistEvent::UpdatedName(newname) => PlaylistData {
                    data: types::Playlist {
                        collaborative: state.data.collaborative,
                        followers: state.data.followers,
                        public: state.data.public,
                        description: state.data.description.clone(),
                        id: state.data.id.clone(),
                        name: newname.to_owned(),
                        owner: state.data.owner.clone(),
                        tracks: state.data.tracks.clone(),
                    },
                    generation: state.generation + 1,
                },
                PlaylistEvent::UpdatedDesciption(newdes) => PlaylistData {
                    data: types::Playlist {
                        collaborative: state.data.collaborative,
                        followers: state.data.followers,
                        public: state.data.public,
                        description: newdes.to_owned(),
                        id: state.data.id.clone(),
                        name: state.data.name.clone(),
                        owner: state.data.owner.clone(),
                        tracks: state.data.tracks.clone(),
                    },
                    generation: state.generation + 1,
                },
                PlaylistEvent::UpdatedTracks(_tracks) => todo!(),
                PlaylistEvent::DeletedPlaylist() => todo!(),
            };
            Ok(state)
        }
        fn handle_command(_state: &Self::State, cmd: &Self::Command) -> Result<Vec<Self::Event>> {
            // SHOULD DO: validate state and command
            // if validation passes...
            let evts = match cmd {
                PlaylistCommand::CreatePlaylist(playlist) => {
                    vec![PlaylistEvent::CreatedPlaylist(playlist.to_owned())]
                }
                PlaylistCommand::UpdateName(newname) => {
                    vec![PlaylistEvent::UpdatedName(newname.to_owned())]
                }
                PlaylistCommand::UpdateDesciption(newdes) => {
                    vec![PlaylistEvent::UpdatedDesciption(newdes.to_owned())]
                }
                PlaylistCommand::UpdateTracks(tracks) => {
                    vec![PlaylistEvent::UpdatedTracks(tracks.to_owned())]
                }
                PlaylistCommand::DeletePlaylist() => {
                    vec![PlaylistEvent::DeletedPlaylist()]
                }
            };
            Ok(evts)
        }
    }
}
