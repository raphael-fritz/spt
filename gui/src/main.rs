use gui::{Controller, MainWindow, UIEvents, User, Users};
use rspotify::{model, prelude::BaseClient};
use slint::{ComponentHandle, ModelRc, PlatformError, VecModel};
use std::{error::Error, sync::mpsc};

const USER_FILE: &str = "data/users.json";
const DATA_FILE: &str = "data/events.json";

pub struct Application {
    _controller: Controller,
    gui: MainWindow,
}

impl Application {
    pub fn new() -> Result<Self, PlatformError> {
        let gui = MainWindow::new()?;
        let (tx, rx) = mpsc::channel::<UIEvents>();
        let _controller = Controller::new(gui.as_weak(), rx);

        // Authenticate with OAuth
        let spotify = match spt::login::login() {
            Ok(spotify) => spotify,
            Err(why) => {
                eprintln!("Login failed: {why}");
                std::process::exit(1)
            }
        };

        // Load Events
        let eventstore =
            spt::eventsourcing::eventstore::JSONEventStore::from_file(DATA_FILE).unwrap();

        // Load Users and initialize User Model
        let users = Users::new(gui.as_weak()).unwrap();
        for user in spt::load_users(USER_FILE).unwrap() {
            if let Ok(user) = spotify.user(model::UserId::from_id_or_uri(&user.id).unwrap()) {
                let playlists: Vec<model::SimplifiedPlaylist> =
                    spotify.user_playlists(user.id.clone()).flatten().collect();
                if let Ok(user) = User::from_spotify(user, playlists, &eventstore) {
                    users.add_user(user).unwrap();
                }
            }
        }

        // Initialize Event Model
        let events = VecModel::default();
        gui.set_events(ModelRc::new(events));

        // Close Requested callback
        {
            let tx = tx.clone();
            gui.window().on_close_requested(move || {
                tx.send(UIEvents::CloseRequested).unwrap();
                slint::CloseRequestResponse::HideWindow
            });
        }

        // Update Button callback
        {
            let tx = tx.clone();
            gui.on_update(move || tx.send(UIEvents::Update).unwrap());
        }

        // User Clicked callback
        {
            let tx = tx.clone();
            gui.on_user_clicked(move |userid| {
                tx.send(UIEvents::UserClicked(userid)).unwrap();
            });
        }

        Ok(Self { _controller, gui })
    }

    pub fn run(&self) -> Result<(), slint::PlatformError> {
        self.gui.run()
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    Ok(Application::new()?.run()?)
}
