use bevy::prelude::*;
use sqlx::{Row, FromRow};
use bevy_sqlx::{SqlxPlugin, SqlxFetch, SqlxComponent};

#[derive(Component, FromRow)]
struct Example {
    id: u32,
    text: String,
    flag: bool,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(SqlxPlugin::default()
            .register_component::<Example>())
        .add_systems(Startup, send_fetch)
        .add_systems(Update, query_spawned)
        .run();
}

fn send_fetch(mut events: EventWriter<SqlxFetch>) {
    // events.send(SqlxFetch { query: "DELETE FROM examples".into() });
    events.send(SqlxFetch { query: "SELECT id, text, flag FROM examples".into() });
    events.send(SqlxFetch { query: "INSERT INTO examples (id, text, flag) VALUES (8, 'hello world', 0)".into() });
    events.send(SqlxFetch { query: "SELECT id, text, flag FROM examples".into() });
    events.send(SqlxFetch { query: "DELETE FROM examples".into() });
}

fn query_spawned(mut query: Query<&SqlxComponent>) {
    for sqlx in &mut query {
        for row in &sqlx.rows {
            dbg!(
                row.get::<u32, _>("id"),
                row.get::<String, _>("text"),
                row.get::<bool, _>("flag"));
        }
    }
}
