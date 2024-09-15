#![feature(trivial_bounds)]
use std::env;
use std::marker::Unpin;
use bevy::prelude::*;
use bevy::tasks::{block_on, AsyncComputeTaskPool, Task};
use bevy::tasks::futures_lite::future;
use sqlx::{
    Error,
    FromRow,
    sqlite::SqliteRow,
    sqlite::SqlitePool,
};

#[derive(Resource, Debug)]
pub struct SqlxDatabase {
    pub pool: SqlitePool
}

#[derive(Resource)]
pub struct SqlxFetchTasks<C: Component + for<'r> FromRow<'r, SqliteRow>> {
    pub tasks: Vec<(String, Task<Result<Vec<C>, Error>>)>,
}


#[derive(Event, Debug)]
pub struct SqlxFetchEvent {
    pub query: String
}

impl SqlxFetchEvent {
    pub fn query(string: &str) -> Self {
        SqlxFetchEvent {
            query: string.to_string()
        }
    }
}

#[derive(Component)]
pub struct SqlxQuery(String);

#[derive(Default)]
pub struct SqlxPlugin<C: Component>(std::marker::PhantomData<C>);

impl<C: std::fmt::Debug + Component + Unpin + for<'r> FromRow<'r, SqliteRow>> Plugin for SqlxPlugin<C> {
    fn build(&self, app: &mut App) {
        let pool = bevy::tasks::block_on(async {
            let env = env::var("DATABASE_URL").unwrap();
            SqlitePool::connect(&env).await.unwrap()
        });
        app.insert_resource(SqlxDatabase { pool });
        app.insert_resource(SqlxFetchTasks::<C> { tasks: Vec::new() });
        app.add_event::<SqlxFetchEvent>();
        app.add_systems(Update, (Self::fetch, Self::spawn));
    }
}

impl<C: std::fmt::Debug + Component + Unpin + for<'r> FromRow<'r, SqliteRow>> SqlxPlugin<C> {
    pub fn fetch(
        database: Res<SqlxDatabase>,
        mut fetch: ResMut<SqlxFetchTasks<C>>,
        mut events: EventReader<SqlxFetchEvent>,
    ) {
        for fetch_event in events.read() {
            let task_pool = AsyncComputeTaskPool::get();
            let db = database.pool.clone();
            let query = fetch_event.query.clone();
            let q = query.clone();
            let task = task_pool.spawn(async move {
                sqlx::query_as(&query).fetch_all(&db).await
            });
            dbg!("fetching", &q, &task);
            fetch.tasks.push((q, task));
        }
    }

    pub fn spawn(
        mut commands: Commands,
        mut fetch: ResMut<SqlxFetchTasks<C>>,
    ) {
        fetch.tasks.retain_mut(|(query, task)| {
            let status = block_on(future::poll_once(task));
            let retain = status.is_none();
            if let Some(result) = status {
                match result {
                    Ok(components) => {
                        for component in components {
                            dbg!("spawning", &component);
                            commands.spawn((
                                SqlxQuery(query.clone()),
                                component,
                            ));
                        }
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
