use bevy::prelude::*;
use bevy_sqlx::{
    PrimaryKey, SqlxDatabase, SqlxEvent, SqlxEventStatus, SqlxPlugin,
};
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
        .add_systems(Startup, (reset, insert.after(reset)))
        .add_systems(Update, watch_status)
        .run();
}

fn reset(db: Res<SqlxDatabase<Sqlite>>) {
    bevy::tasks::block_on(async {
        sqlx::query!("DELETE FROM bars").execute(&db.pool).await.unwrap();
        sqlx::query!("DELETE FROM foos").execute(&db.pool).await.unwrap();
    });
}

fn insert(mut events: EventWriter<SqlxEvent<Sqlite, Foo>>) {
    let sql = "INSERT INTO foos(text) VALUES ('insert') RETURNING *";
    events.send(SqlxEvent::<Sqlite, Foo>::query(sql));
}

fn watch_status(mut statuses: EventReader<SqlxEventStatus<Sqlite, Foo>>) {
    for status in statuses.read() {
        dbg!({
            "status";
            status
        });
    }
}
