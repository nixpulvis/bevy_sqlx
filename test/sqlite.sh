cargo sqlx database setup
cargo build --examples --features sqlx/sqlite,bevy/bevy_winit,bevy/wayland
cargo test --features sqlx/sqlite
