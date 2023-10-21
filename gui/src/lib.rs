use rspotify::model;
use slint::{
    EventLoopError, Image, Model, ModelRc, Rgb8Pixel, SharedPixelBuffer, SharedString, VecModel,
    Weak,
};
use spt::eventsourcing::eventstore::JSONEventStore;
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{Receiver, RecvTimeoutError},
        Arc,
    },
    thread,
    time::Duration,
};

slint::include_modules!();

#[derive(Debug)]
pub enum Error {
    EventLoopError(slint::EventLoopError),
    RequestError(reqwest::Error),
    ImageError(image::ImageError),
}

impl From<EventLoopError> for Error {
    fn from(value: EventLoopError) -> Self {
        Self::EventLoopError(value)
    }
}

impl From<reqwest::Error> for Error {
    fn from(value: reqwest::Error) -> Self {
        Self::RequestError(value)
    }
}

impl From<image::ImageError> for Error {
    fn from(value: image::ImageError) -> Self {
        Self::ImageError(value)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum UIEvents {
    CloseRequested,
    Update,
    UserClicked(SharedString),
}

pub struct User {
    id: String,
    events: usize,
    name: Option<String>,
    img: SharedPixelBuffer<Rgb8Pixel>,
}

impl User {
    pub fn new(
        id: &str,
        events: usize,
        name: Option<&str>,
        img: SharedPixelBuffer<Rgb8Pixel>,
    ) -> Self {
        Self {
            id: id.into(),
            events: events,
            name: name.map(|s| s.into()),
            img: img,
        }
    }

    pub fn from_spotify(
        user: model::PublicUser,
        playlists: Vec<model::SimplifiedPlaylist>,
        eventstore: &JSONEventStore,
    ) -> Result<Self> {
        let img = match user.images.first() {
            None => SharedPixelBuffer::new(40, 40),
            Some(img) => {
                let img = reqwest::blocking::get(&img.url)?.bytes()?;
                let img = image::load_from_memory(&img)?
                    .resize(40, 40, image::imageops::Lanczos3)
                    .into_rgb8();
                SharedPixelBuffer::<Rgb8Pixel>::clone_from_slice(img.as_raw(), 40, 40)
            }
        };

        let events: usize = playlists
            .iter()
            .map(|pl| {
                eventstore
                    .get_all::<spt::eventsourcing::domain::PlaylistEvent>(pl.id.to_string())
                    .len()
            })
            .sum();

        Ok(Self {
            id: user.id.to_string(),
            events: events,
            name: user.display_name,
            img,
        })
    }
}

impl From<User> for UserData {
    fn from(value: User) -> Self {
        Self {
            events: value.events as i32,
            id: value.id.into(),
            name: value.name.unwrap_or_default().into(),
            picture: Image::from_rgb8(value.img),
        }
    }
}

pub struct Users {
    handle: Weak<MainWindow>,
}

impl Users {
    pub fn new(handle: Weak<MainWindow>) -> Result<Self> {
        let users = Self { handle };
        users.init()?;
        Ok(users)
    }

    fn init(&self) -> Result<()> {
        Ok(self.handle.upgrade_in_event_loop(move |handle| {
            handle.set_users(ModelRc::new(VecModel::default()))
        })?)
    }

    pub fn add_user(&self, user: User) -> Result<()> {
        Ok(self.handle.upgrade_in_event_loop(move |handle| {
            let users = handle.get_users();
            let users: &VecModel<UserData> = users.as_any().downcast_ref().unwrap();
            users.push(user.into());
        })?)
    }
}

pub struct Controller {
    _handle: Weak<MainWindow>,
    flag: Arc<AtomicBool>,
    thandle: Option<thread::JoinHandle<()>>,
}

impl Controller {
    pub fn new(_handle: Weak<MainWindow>, rx: Receiver<UIEvents>) -> Self {
        let flag = Arc::new(AtomicBool::new(true));
        let thandle = Some({
            let flag = flag.clone();
            let handle = _handle.clone();
            thread::spawn(move || Self::run(flag, rx, handle))
        });

        Controller {
            _handle,
            flag,
            thandle,
        }
    }

    fn run(flag: Arc<AtomicBool>, rx: Receiver<UIEvents>, handle: Weak<MainWindow>) {
        while flag.load(Ordering::Relaxed) {
            let event = match rx.recv_timeout(Duration::from_millis(500)) {
                Ok(val) => val,
                Err(RecvTimeoutError::Timeout) => continue,
                Err(RecvTimeoutError::Disconnected) => break,
            };
            println!("{event:?}");

            if let UIEvents::UserClicked(id) = event {
                let events = vec![(id.into(), false, vec!["Test123".into(), "Test456".into()])];
                Self::update_timeline(handle.clone(), events);
            }
        }
    }

    fn update_timeline(handle: Weak<MainWindow>, new: Vec<(String, bool, Vec<String>)>) {
        handle
            .upgrade_in_event_loop(move |handle| {
                handle.set_events(ModelRc::new(VecModel::from_slice(
                    &new.into_iter()
                        .map(|(date, open, events)| EntryData {
                            date: date.into(),
                            events: ModelRc::new(VecModel::from_slice(
                                &events
                                    .into_iter()
                                    .map(|s| s.into())
                                    .collect::<Vec<SharedString>>(),
                            )),
                            open,
                        })
                        .collect::<Vec<EntryData>>(),
                )));
            })
            .unwrap()
    }
}

impl Drop for Controller {
    fn drop(&mut self) {
        self.flag.store(false, Ordering::Relaxed);
        self.thandle.take().unwrap().join().unwrap();
    }
}
