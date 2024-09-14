use std::env;
use bevy::prelude::*;
use bevy::tasks::{block_on, AsyncComputeTaskPool, Task};
use bevy::tasks::futures_lite::future;
use sqlx::{
    Error,
    sqlite::SqliteRow,
    sqlite::SqlitePool,
};

#[derive(Resource, Debug)]
pub struct SqlxDatabase {
    pub pool: SqlitePool
}

#[derive(Resource)]
pub struct SqlxFetchTasks(Vec<(String, Task<Result<Vec<SqliteRow>, Error>>)>);


#[derive(Event, Debug)]
pub struct SqlxFetch {
    pub query: String
}

#[derive(Component)]
pub struct SqlxComponent {
    pub query: String,
    pub rows: Vec<SqliteRow>,
}

#[derive(Default)]
pub struct SqlxPlugin;

impl Plugin for SqlxPlugin {
    fn build(&self, app: &mut App) {
        let pool = bevy::tasks::block_on(async {
            let env = env::var("DATABASE_URL").unwrap();
            SqlitePool::connect(&env).await.unwrap()
        });
        app.insert_resource(SqlxDatabase { pool });
        app.insert_resource(SqlxFetchTasks(Vec::new()));
        app.add_event::<SqlxFetch>();
        app.add_systems(Update, (Self::fetch, Self::spawn));
    }
}

impl SqlxPlugin {
    pub fn register_component<C: Component>(self) -> Self {
        self
    }

    fn fetch(
        database: Res<SqlxDatabase>,
        mut tasks: ResMut<SqlxFetchTasks>,
        mut events: EventReader<SqlxFetch>,
    ) {
        for fetch in events.read() {
            let task_pool = AsyncComputeTaskPool::get();
            let db = database.pool.clone();
            let query = fetch.query.clone();
            let q = query.clone();
            let task = task_pool.spawn(async move {
                sqlx::query(&query).fetch_all(&db).await
            });
            tasks.0.push((q, task));
        }
    }

    fn spawn(
        mut commands: Commands,
        mut tasks: ResMut<SqlxFetchTasks>
    ) {
        tasks.0.retain_mut(|(query, task)| {
            let status = block_on(future::poll_once(task));
            let retain = status.is_none();
            if let Some(result) = status {
                match result {
                    Ok(rows) => {
                        commands.spawn(SqlxComponent {
                            query: query.clone(),
                            rows
                        });
                    }
                    Err(err) => {
                        dbg!(err);
                    }
                }
            }
            retain
        });
    }
}
