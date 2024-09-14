use std::env;
use bevy::prelude::*;
use bevy::tasks::{block_on, AsyncComputeTaskPool, Task};
use bevy::tasks::futures_lite::future;
use sqlx::{SqlitePool, query::Query};

#[derive(Resource)]
pub struct SqlxDatabase {
    pub pool: SqlitePool
}

#[derive(Resource)]
pub struct SqlxFetchTasks {
    tasks: Vec<Task<i32>>
}

#[derive(Event, Debug)]
pub struct SqlxFetch {
    pub query: String
}

pub struct SqlxPlugin;

impl Plugin for SqlxPlugin {
    fn build(&self, app: &mut App) {
        let pool = bevy::tasks::block_on(async {
            let env = env::var("DATABASE_URL").unwrap();
            SqlitePool::connect(&env).await.unwrap()
        });
        app.insert_resource(SqlxDatabase { pool });
        app.insert_resource(SqlxFetchTasks { tasks: vec![] });
        app.add_event::<SqlxFetch>();
        app.add_systems(Update, Self::fetch);
    }
}

impl SqlxPlugin {
    fn fetch(
        database: Res<SqlxDatabase>,
        mut events: EventReader<SqlxFetch>,
    ) {
        for fetch in events.read() {
            dbg!(&fetch);

            let task_pool = AsyncComputeTaskPool::get();
            let db = database.pool.clone();
            let query = fetch.query.clone();
            let task = task_pool.spawn(async move {
                sqlx::query(&query).fetch_all(&db).await
            });

        }
    }

    fn spawn(
        database: Res<SqlxDatabase>,
    ) {
        // TODO:
    }
}
