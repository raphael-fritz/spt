use gui::{Controller, MainWindow, UIEvents, UserData};
use slint::{ComponentHandle, Image, ModelRc, PlatformError, SharedPixelBuffer, VecModel};
use std::{error::Error, sync::mpsc};

pub struct Application {
    _controller: Controller,
    gui: MainWindow,
}

impl Application {
    pub fn new() -> Result<Self, PlatformError> {
        let gui = MainWindow::new()?;
        let (tx, rx) = mpsc::channel::<UIEvents>();
        let _controller = Controller::new(gui.as_weak(), rx);

        // Initialize User and Event Models
        let imgbuf = SharedPixelBuffer::new(40, 40);
        let user = UserData {
            id: "Test".into(),
            events: 0,
            name: "TestUser".into(),
            picture: Image::from_rgb8(imgbuf),
        };
        let users = VecModel::default();
        users.push(user);
        let events = VecModel::default();
        gui.set_users(ModelRc::new(users));
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
