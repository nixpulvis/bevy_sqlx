use bevy::prelude::*;
use bevy::app::ScheduleRunnerPlugin;
use sqlx::Sqlite;
use bevy_sqlx::{SqlxPlugin, SqlxDatabase, component::SqlxDummy};

fn main() {
    let mut app= App::new();
    app.add_plugins(MinimalPlugins.set(ScheduleRunnerPlugin::run_once()));
    app.add_plugins(SqlxPlugin::<Sqlite, SqlxDummy>::url("sqlite:db/sqlite.db"));
    let db = app.world().get_resource::<SqlxDatabase<Sqlite>>().unwrap();

    let record = bevy::tasks::block_on(async {
        sqlx::query!("SELECT (1) as id, 'test' as text")
            .fetch_one(&db.pool)
            .await.unwrap()
    });

    dbg!(record);
}
