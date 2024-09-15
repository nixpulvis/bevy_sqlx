use bevy::prelude::*;
use sqlx::FromRow;
use bevy_sqlx::{SqlxPlugin, SqlxPrimaryKey, SqlxEvent};

/// ### SQL Table Schema
///
/// CREATE TABLE foos (
///     id    INTEGER   PRIMARY KEY,
///     text  TEXT      NOT NULL,
///     flag  BOOLEAN   NOT NULL DEFAULT 0
/// );
#[derive(Component, FromRow, Debug)]
struct Foo {
    id: u32,
    flag: bool,
    text: String,
}

impl SqlxPrimaryKey for Foo {
    type Column = u32;

    fn id(&self) -> Self::Column {
        self.id
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(SqlxPlugin::<Foo>::default())
        .add_systems(Startup, (delete, insert.after(delete)))
        .add_systems(Update, query)
        .run();
}

fn delete(mut events: EventWriter<SqlxEvent<Foo>>) {
    SqlxEvent::<Foo>::query("DELETE FROM foos")
        .send(&mut events);
}

fn insert(mut events: EventWriter<SqlxEvent<Foo>>) {
    let sql = "INSERT INTO foos(text) VALUES ('insert') RETURNING *";
    SqlxEvent::<Foo>::query(sql)
        .send(&mut events);
}

fn query(foos: Query<Ref<Foo>>) {
    for foo in &foos {
        if foo.is_added() {
            dbg!(foo);
        }
    }
}
