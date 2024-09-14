use bevy::prelude::*;
use bevy_sqlx::{SqlxPlugin, SqlxFetch};

#[derive(Component)]
struct Example {
    id: u32,
    text: String,
    flag: bool,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(SqlxPlugin)
        .add_systems(Startup, send_fetch)
        .add_systems(Update, query_spawned)
        .run();
}

fn send_fetch(mut events: EventWriter<SqlxFetch>) {
    events.send(SqlxFetch { query: "SELECT id, text, flag FROM examples".into() });
}

fn query_spawned(query: Query<Entity>) {
    // dbg!(query.single());
}
