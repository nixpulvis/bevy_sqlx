use bevy::prelude::*;
use bevy_sqlx::{PrimaryKey, SqlxEvent, SqlxPlugin};
use sqlx::{FromRow, Postgres};

#[derive(Component, FromRow, Debug)]
#[allow(unused)]
struct Foo {
    id: i32,
    text: String,
}

impl PrimaryKey for Foo {
    type Column = i32;

    fn primary_key(&self) -> Self::Column {
        self.id
    }
}

fn main() {
    let url = "postgres://localhost/bevy_sqlx";
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(SqlxPlugin::<Postgres, Foo>::from_url(url))
        .add_systems(Startup, (delete, insert.after(delete)))
        .add_systems(Update, query)
        .run();
}

fn delete(mut events: EventWriter<SqlxEvent<Postgres, Foo>>) {
    events.send(SqlxEvent::<Postgres, Foo>::query("DELETE FROM foos"));
}

fn insert(mut events: EventWriter<SqlxEvent<Postgres, Foo>>) {
    let sql = "INSERT INTO foos(id, text) VALUES (1, 'insert') RETURNING *";
    events.send(SqlxEvent::<Postgres, Foo>::query(sql));
}

fn query(foos: Query<Ref<Foo>>) {
    for foo in &foos {
        if foo.is_added() {
            dbg!(foo);
        }
    }
}
