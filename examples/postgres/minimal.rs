use bevy::prelude::*;
use sqlx::{FromRow, Postgres};
use bevy_sqlx::{SqlxPlugin, PrimaryKey, SqlxEvent};

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
        .add_plugins(SqlxPlugin::<Postgres, Foo>::url(url))
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
