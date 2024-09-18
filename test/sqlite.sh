#!/bin/sh
cargo sqlx database setup
cargo build &&
cargo build --examples --features sqlx/sqlite,bevy/bevy_winit,bevy/wayland &&
cargo test --features sqlx/sqlite
