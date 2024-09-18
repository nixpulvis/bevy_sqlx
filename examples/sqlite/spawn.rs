use bevy::prelude::*;
use bevy_sqlx::{
    PrimaryKey, SqlxColumn, SqlxEvent, SqlxEventStatus, SqlxPlugin, ToRow,
};
use sqlx::{FromRow, Sqlite};

#[derive(Component, FromRow, Debug)]
#[allow(unused)]
struct Foo {
    id: u32,
    text: String,
    flag: bool,
}

impl PrimaryKey for Foo {
    fn primary_key(&self) -> SqlxColumn {
        SqlxColumn::new("id", self.id.to_string())
    }
}

impl ToRow for Foo {
    fn to_row(&self) -> Vec<SqlxColumn> {
        vec![
            SqlxColumn::new("id", self.id.to_string()),
            SqlxColumn::new("text", self.text.to_string()),
            SqlxColumn::new("flag", self.flag.to_string()),
        ]
    }
}

fn main() {
    let url = "sqlite:db/sqlite.db";
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(SqlxPlugin::<Sqlite, Foo>::from_url(url))
        .add_systems(Startup, spawn)
        .add_systems(Update, watch_status)
        .run();
}

fn spawn(mut commands: Commands) {
    let foo = Foo { id: 0, text: "spawned".into(), flag: true };
    commands.spawn(foo);
}

fn watch_status(mut statuses: EventReader<SqlxEventStatus>) {
    dbg!("HIT");
    for status in statuses.read() {
        dbg!({
            "SqlxEventStatus";
            status
        });
    }
}
