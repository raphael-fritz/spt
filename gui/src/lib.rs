use slint::{ModelRc, SharedString, VecModel, Weak};
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
pub enum UIEvents {
    CloseRequested,
    Update,
    UserClicked(SharedString),
}

pub struct Controller {
    handle: Weak<MainWindow>,
    flag: Arc<AtomicBool>,
    thandle: Option<thread::JoinHandle<()>>,
}

impl Controller {
    pub fn new(handle: Weak<MainWindow>, rx: Receiver<UIEvents>) -> Self {
        let flag = Arc::new(AtomicBool::new(true));
        let thandle = Some({
            let flag = flag.clone();
            let handle = handle.clone();
            thread::spawn(move || Self::run(flag, rx, handle))
        });

        Controller {
            handle,
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
