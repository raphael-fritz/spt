use gui::{Controller, MainWindow, UIEvents, User, Users};
use slint::{ComponentHandle, ModelRc, PlatformError, SharedPixelBuffer, VecModel};
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
        let users = Users::new(gui.as_weak()).unwrap();
        let user = User::new("Test", 0, Some("TestUser"), SharedPixelBuffer::new(40, 40));
        users.add_user(user).unwrap();

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
