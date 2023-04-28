# Spotify Playlist Tracker

Track a Spotify playlist on a regular basis with an model powered by event sourcing!

This application is in very early development and isn't doing much useful stuff yet.

## To-do

- [x] Only track playlists owned by the user
- [x] Compare playlist snapshot id before comparing all tracks
- [ ] Handle errors (especially in Playlist.from_id)
- [ ] GUI to analyze data
- [ ] Don't emit AddedTracks/RemovedTracks event if only track details have changed (e.g. if the name of a track changes)

## Useful links

- [Eventsourcing Github](https://github.com/pholactery/eventsourcing)
- [Eventsourcing Docs](https://docs.rs/eventsourcing/latest/eventsourcing/)
- [Eventsourcing Examples](https://github.com/pholactery/eventsourcing/tree/master/examples)
- [Event Sourcing with Aggregates](https://medium.com/capital-one-tech/event-sourcing-with-aggregates-in-rust-4022af41cf67)
- ["impl trait for vec"](https://github.com/apolitical/impl-display-for-vec)
- ["Indicatif"](https://docs.rs/indicatif/latest/indicatif/)
