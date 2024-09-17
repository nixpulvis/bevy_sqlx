DATABASE_URL=postgres://localhost/bevy_sqlx cargo sqlx database setup
cargo build --examples --features sqlx/postgres,bevy/bevy_winit,bevy/wayland
cargo test --features sqlx/postgres
