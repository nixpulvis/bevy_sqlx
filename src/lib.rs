#![feature(trivial_bounds)]
use std::env;
use std::fmt::Debug;
use std::marker::{PhantomData, Unpin};
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
pub struct SqlxFetchTasks<C: Component + Clone + for<'r> FromRow<'r, SqliteRow>> {
    pub tasks: Vec<(String, Task<Result<Vec<C>, Error>>)>,
}


#[derive(Event, Debug, Clone)]
pub struct SqlxEvent<C: Component + Clone + for<'r> FromRow<'r, SqliteRow>> {
    pub query: String,
    _c: PhantomData<C>,
}

impl<C: Component + Clone + for<'r> FromRow<'r, SqliteRow>> SqlxEvent<C> {
    pub fn query(string: &str) -> Self {
        SqlxEvent {
            query: string.to_string(),
            _c: PhantomData,
        }
    }

    pub fn send(self, events: &mut EventWriter<SqlxEvent<C>>) -> Self {
        events.send(self.clone());
        self
    }

    pub fn trigger(self, commands: &mut Commands) -> Self {
        commands.trigger(self.clone());
        self
    }

    // pub fn bind<T>(self, value: T) -> Self {
    //     self
    // }
}

#[derive(Component)]
pub struct SqlxQuery(pub String);

#[derive(Default)]
pub struct SqlxPlugin<C: Component>(PhantomData<C>);

impl<C: Debug + Component + Clone + Unpin + for<'r> FromRow<'r, SqliteRow>> Plugin for SqlxPlugin<C> {
    fn build(&self, app: &mut App) {
        let pool = bevy::tasks::block_on(async {
            let env = env::var("DATABASE_URL").unwrap();
            SqlitePool::connect(&env).await.unwrap()
        });
        app.insert_resource(SqlxDatabase { pool });
        app.insert_resource(SqlxFetchTasks::<C> { tasks: Vec::new() });
        app.add_event::<SqlxEvent<C>>();
        app.add_systems(Update, (Self::fetch, Self::spawn));
    }
}

impl<C: Debug + Component + Clone + Unpin + for<'r> FromRow<'r, SqliteRow>> SqlxPlugin<C> {
    pub fn fetch(
        database: Res<SqlxDatabase>,
        mut fetch: ResMut<SqlxFetchTasks<C>>,
        mut events: EventReader<SqlxEvent<C>>,
    ) {
        for fetch_event in events.read() {
            let task_pool = AsyncComputeTaskPool::get();
            let db = database.pool.clone();
            let query = fetch_event.query.clone();
            let q = query.clone();
            let task = task_pool.spawn(async move {
                sqlx::query_as(&query).fetch_all(&db).await
            });
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
