use slint::{PlatformError, Weak};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, RecvTimeoutError},
        Arc,
    },
    thread,
    time::Duration,
};

slint::include_modules!();

#[derive(Debug)]
enum UIEvents {
    CloseRequested,
    Update,
    // ShowUserEvents,
}

struct Controller {
    handle: Weak<MainWindow>,
    flag: Arc<AtomicBool>,
    thandle: Option<thread::JoinHandle<()>>,
}

impl Controller {
    pub fn new(handle: Weak<MainWindow>, rx: Receiver<UIEvents>) -> Self {
        let flag = Arc::new(AtomicBool::new(true));
        let thandle = Some({
            let flag = flag.clone();
            thread::spawn(move || Self::run(flag, rx))
        });

        Controller {
            handle,
            flag,
            thandle,
        }
    }

    fn run(flag: Arc<AtomicBool>, rx: Receiver<UIEvents>) {
        while flag.load(Ordering::Relaxed) {
            let event = match rx.recv_timeout(Duration::from_millis(500)) {
                Ok(val) => val,
                Err(RecvTimeoutError::Timeout) => continue,
                Err(RecvTimeoutError::Disconnected) => break,
            };
            println!("{event:?}");
        }
    }
}

impl Drop for Controller {
    fn drop(&mut self) {
        self.flag.store(false, Ordering::Relaxed);
        self.thandle.take().unwrap().join().unwrap();
    }
}

pub struct Application {
    controller: Controller,
    gui: MainWindow,
}

impl Application {
    pub fn new() -> Result<Self, PlatformError> {
        let gui = MainWindow::new()?;
        let (tx, rx) = mpsc::channel::<UIEvents>();
        let controller = Controller::new(gui.as_weak(), rx);

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

        Ok(Self { controller, gui })
    }

    pub fn run(&self) -> Result<(), slint::PlatformError> {
        self.gui.run()
    }
}
