use chrono::prelude::{DateTime, Utc};
use rspotify::{model, prelude::BaseClient, ClientResult};
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub enum SPTError {
    Authentication(crate::login::AuthenticationError),
    Parse(serde_json::Error),
    Client(rspotify::ClientError),
    IO(std::io::Error),
    EventSourcing(crate::eventsourcing::Error),
}

impl std::fmt::Display for SPTError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            SPTError::IO(err) => write!(f, "{}", err),
            SPTError::Parse(err) => write!(f, "{}", err),
            SPTError::Client(err) => write!(f, "{}", err),
            SPTError::Authentication(err) => write!(f, "{}", err),
            SPTError::EventSourcing(err) => write!(f, "{}", err),
        }
    }
}

impl From<crate::login::AuthenticationError> for SPTError {
    fn from(err: crate::login::AuthenticationError) -> Self {
        SPTError::Authentication(err)
    }
}

impl From<std::io::Error> for SPTError {
    fn from(err: std::io::Error) -> Self {
        SPTError::IO(err)
    }
}

impl From<serde_json::Error> for SPTError {
    fn from(err: serde_json::Error) -> Self {
        SPTError::Parse(err)
    }
}

impl From<rspotify::ClientError> for SPTError {
    fn from(err: rspotify::ClientError) -> Self {
        SPTError::Client(err)
    }
}

impl From<crate::eventsourcing::Error> for SPTError {
    fn from(err: crate::eventsourcing::Error) -> Self {
        SPTError::EventSourcing(err)
    }
}

impl std::error::Error for SPTError {}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct User {
    pub display_name: Option<String>,
    pub id: String,
}
impl User {
    /// Creates new empty user
    pub fn new() -> User {
        User {
            display_name: None,
            id: String::new(),
        }
    }
    pub fn name_or_id(&self) -> &str {
        self.display_name.as_ref().unwrap_or(&self.id)
    }
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

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
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

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
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

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
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

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
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

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
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

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
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

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct PlaylistItems(pub Vec<PlaylistItem>);
impl std::ops::Deref for PlaylistItems {
    type Target = Vec<PlaylistItem>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl From<Vec<model::PlaylistItem>> for PlaylistItems {
    fn from(items: Vec<model::PlaylistItem>) -> Self {
        PlaylistItems(
            items
                .into_iter()
                .map(|item| PlaylistItem::from(item))
                .collect(),
        )
    }
}
impl FromIterator<PlaylistItem> for PlaylistItems {
    fn from_iter<T: IntoIterator<Item = PlaylistItem>>(iter: T) -> Self {
        PlaylistItems(iter.into_iter().collect())
    }
}
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct Playlist {
    pub collaborative: bool,
    pub description: Option<String>,
    pub followers: u32,
    pub id: String,
    pub name: String,
    pub owner: User,
    pub public: Option<bool>,
    pub tracks: PlaylistItems,
    pub snapshot_id: String,
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
            snapshot_id: item.snapshot_id,
        }
    }
}

impl Playlist {
    pub fn from_id(
        client: &rspotify::AuthCodeSpotify,
        playlist_id: model::PlaylistId,
        fields: Option<&str>,
        market: Option<rspotify::model::Market>,
    ) -> Result<Playlist, rspotify::ClientError> {
        let playlist = client.playlist(playlist_id, fields, market)?;
        let tracks: Vec<ClientResult<rspotify::model::PlaylistItem>> = client
            .playlist_items(playlist.id.clone(), fields, market)
            .collect();
        let tracks: ClientResult<Vec<model::PlaylistItem>> = tracks.into_iter().collect();
        let mut playlist = Playlist::from(playlist);
        playlist.tracks = PlaylistItems::from(tracks?);
        return Ok(playlist);
    }

    /// Creates new empty Playlist
    pub fn new() -> Playlist {
        Playlist {
            collaborative: false,
            description: None,
            followers: 0,
            id: String::new(),
            name: String::new(),
            owner: User::new(),
            public: None,
            tracks: PlaylistItems(vec![]),
            snapshot_id: String::new(),
        }
    }
}
