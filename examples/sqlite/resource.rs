use bevy::app::ScheduleRunnerPlugin;
use bevy::prelude::*;
use bevy_sqlx::{component::SqlxDummy, SqlxDatabase, SqlxPlugin};
use sqlx::Sqlite;

fn main() {
    App::new()
        .add_plugins(MinimalPlugins.set(ScheduleRunnerPlugin::run_once()))
        .add_plugins(SqlxPlugin::<Sqlite, SqlxDummy>::from_url(
            "sqlite:db/sqlite.db",
        ))
        .add_systems(Startup, select)
        .update();
}

fn select(db: Res<SqlxDatabase<Sqlite>>) {
    let record = bevy::tasks::block_on(async {
        sqlx::query("SELECT (1) as id, 'test' as text")
            .fetch_one(&db.pool)
            .await
            .unwrap()
    });
    dbg!(record);
}
