use bevy::prelude::*;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use sqlx::FromRow;
use bevy_sqlx::{SqlxPlugin, SqlxPrimaryKey, SqlxEvent};

#[derive(Component, FromRow, Debug)]
struct MyTable {
    id: u32,
    flag: bool,
    text: String,
}

impl SqlxPrimaryKey for MyTable {
    type Column = u32;

    fn id(&self) -> Self::Column {
        self.id
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(WorldInspectorPlugin::new())
        .add_plugins(SqlxPlugin::<MyTable>::default())
        .add_systems(Startup, insert)
        .add_systems(Update, query)
        .run();
}

fn insert(
    mut commands: Commands,
    mut events: EventWriter<SqlxEvent<MyTable>>,
) {
    let sql = "INSERT INTO foos(text) VALUES ('insert') RETURNING *";
    SqlxEvent::<MyTable>::query(sql)
        .send(&mut events)
        .trigger(&mut commands);
}

fn query(my_tables: Query<&MyTable>) {
    for my_table in &my_tables {
        dbg!(my_table);
    }
}
