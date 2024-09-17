DATABASE_URL=sqlite:db/bevy_sqlx.db         cargo sqlx database setup
DATABASE_URL=postgres://localhost/bevy_sqlx cargo sqlx database setup

cargo build --examples --features sqlx/sqlite,bevy/bevy_winit,bevy/wayland
cargo build --examples --features sqlx/postgres,bevy/bevy_winit,bevy/wayland
cargo test --features sqlx/sqlite
