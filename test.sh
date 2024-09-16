cargo test --features sqlx/sqlite
cargo build --examples --features sqlx/sqlite,bevy/bevy_winit,bevy/wayland
cargo build --examples --features sqlx/postgres,bevy/bevy_winit,bevy/wayland
