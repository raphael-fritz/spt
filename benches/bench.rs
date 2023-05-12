#![feature(test)]
use spt::eventsourcing::domain;
use spt::eventsourcing::eventstore::JSONEventStore;
extern crate test;

const PLAYLISTS: &[&str] = &[
    "spotify:playlist:0yy8wqpMt8v7CJBkZGEve6",
    "spotify:playlist:4REFftIedZ7P0lXeAVtul6",
    "spotify:playlist:4hfH5nkiAuFbF3xd8BvnR6",
    "spotify:playlist:43LYgPshFoeyjRhENW70e3",
    "spotify:playlist:1HvUh6hc7j8l2Ckb3kxmcB",
    "spotify:playlist:4OAOYbzgCOOHUBIbEzlZq4",
    "spotify:playlist:0RP9QRO9xPA0EMBC5x3zLC",
    "spotify:playlist:4TTAXEGshokRCp5LYhMPT0",
    "spotify:playlist:2SOTPUb5Nxej7z9TvZitMH",
    "spotify:playlist:590ACVLTiS4KmqzLzWZRn9",
    "spotify:playlist:0Ku4q2Z4mVf7xuAHE8lBRM",
    "spotify:playlist:4SCvA6FoJU65wKBiVm7Iuz",
];
const DATA_DIR: &str = "benches/data";
const DATA_FILE: &str = "events";

#[cfg(test)]
mod tests {
    use super::*;
    use test::Bencher;

    fn get_eventstore() -> JSONEventStore {
        JSONEventStore::from_file(&format!("{}/{}.json", DATA_DIR, DATA_FILE))
            .expect("Event Data should be present to run benchmarks")
    }

    #[bench]
    fn bench_build_local(b: &mut Bencher) {
        let event_store = get_eventstore();
        b.iter(|| {
            let pl = PLAYLISTS[0];
            spt::build_local(&pl.to_string(), &event_store).unwrap()
        })
    }

    #[bench]
    fn bench_get_all(b: &mut Bencher) {
        let event_store = get_eventstore();

        b.iter(|| {
            let pl = PLAYLISTS[0];
            event_store.get_all::<domain::PlaylistEvent>(pl.to_string())
        })
    }
}
