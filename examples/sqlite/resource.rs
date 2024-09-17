use bevy::app::ScheduleRunnerPlugin;
use bevy::prelude::*;
use bevy_sqlx::{component::SqlxDummy, SqlxDatabase, SqlxPlugin};
use sqlx::Sqlite;

fn main() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins.set(ScheduleRunnerPlugin::run_once()));
    app.add_plugins(SqlxPlugin::<Sqlite, SqlxDummy>::from_url(
        "sqlite:db/sqlite.db",
    ));
    let db = app.world().get_resource::<SqlxDatabase<Sqlite>>().unwrap();

    let record = bevy::tasks::block_on(async {
        sqlx::query!("SELECT (1) as id, 'test' as text")
            .fetch_one(&db.pool)
            .await
            .unwrap()
    });

    dbg!(record);
}
