use bevy::prelude::*;
use sqlx::{FromRow, Sqlite};
use bevy_sqlx::{SqlxPlugin, SqlxPrimaryKey, SqlxEvent};

#[derive(Component, FromRow, Debug)]
#[allow(unused)]
struct Foo {
    id: u32,
    text: String,
}

impl SqlxPrimaryKey for Foo {
    type Column = u32;

    fn id(&self) -> Self::Column {
        self.id
    }
}

fn main() {
    let url = "sqlite:db/sqlite.db";
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(SqlxPlugin::<Sqlite, Foo>::url(url))
        .add_systems(Startup, (delete, insert.after(delete)))
        .add_systems(Update, query)
        .run();
}

fn delete(mut events: EventWriter<SqlxEvent<Sqlite, Foo>>) {
    SqlxEvent::<Sqlite, Foo>::query("DELETE FROM foos")
        .send(&mut events);
}

fn insert(mut events: EventWriter<SqlxEvent<Sqlite, Foo>>) {
    let sql = "INSERT INTO foos(text) VALUES ('insert') RETURNING *";
    SqlxEvent::<Sqlite, Foo>::query(sql)
        .send(&mut events);
}

fn query(foos: Query<Ref<Foo>>) {
    for foo in &foos {
        if foo.is_added() {
            dbg!(foo);
        }
    }
}
