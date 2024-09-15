use std::env;
use bevy::prelude::*;
use sqlx::FromRow;
use sqlx::{Postgres, PgPool};
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
    id: i32,
    flag: bool,
    text: String,
}

impl SqlxPrimaryKey for Foo {
    type Column = i32;

    fn id(&self) -> Self::Column {
        self.id
    }
}

fn main() {
    let url = env::var("DATABASE_URL").unwrap();
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(SqlxPlugin::<Postgres, Foo>::url(&url))
        .add_systems(Startup, (delete, insert.after(delete)))
        .add_systems(Update, query)
        .run();
}

fn delete(mut events: EventWriter<SqlxEvent<Postgres, Foo>>) {
    SqlxEvent::<Postgres, Foo>::query("DELETE FROM foos")
        .send(&mut events);
}

fn insert(mut events: EventWriter<SqlxEvent<Postgres, Foo>>) {
    let sql = "INSERT INTO foos(id, text) VALUES (1, 'insert') RETURNING *";
    SqlxEvent::<Postgres, Foo>::query(sql)
        .send(&mut events);
}

fn query(foos: Query<Ref<Foo>>) {
    for foo in &foos {
        if foo.is_added() {
            dbg!(foo);
        }
    }
}
