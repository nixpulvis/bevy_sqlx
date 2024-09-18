use bevy::prelude::*;
use bevy_sqlx::{PrimaryKey, SqlxEvent, SqlxPlugin};
use sqlx::{FromRow, Sqlite};

#[derive(Component, FromRow, Debug)]
#[allow(unused)]
struct Foo {
    id: u32,
    text: String,
}

impl PrimaryKey for Foo {
    type Column = u32;

    fn primary_key(&self) -> Self::Column {
        self.id
    }
}

fn main() {
    let url = "sqlite:db/sqlite.db";
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(SqlxPlugin::<Sqlite, Foo>::from_url(url))
        .add_systems(Startup, (delete, insert.after(delete)))
        .add_systems(Update, query)
        .run();
}

fn delete(mut events: EventWriter<SqlxEvent<Sqlite, Foo>>) {
    events.send(SqlxEvent::<Sqlite, Foo>::query_sync("DELETE FROM foos"));
}

fn insert(mut events: EventWriter<SqlxEvent<Sqlite, Foo>>) {
    let sql = "INSERT INTO foos(text) VALUES ('insert') RETURNING *";
    events.send(SqlxEvent::<Sqlite, Foo>::query_sync(sql));
}

fn query(foos: Query<Ref<Foo>>) {
    for foo in &foos {
        if foo.is_added() {
            dbg!(foo);
        }
    }
}
