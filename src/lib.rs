#![feature(trivial_bounds)]
use std::env;
use std::fmt::Debug;
use std::marker::{PhantomData, Unpin};
use bevy::prelude::*;
use bevy::ecs::system::SystemState;
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
pub struct SqlxTasks<C: Component + Clone + for<'r> FromRow<'r, SqliteRow>>
(pub Vec<(String, Task<Result<Vec<C>, Error>>)>);


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

pub trait SqlxPrimaryKey {
    type Column: PartialEq;
    fn id(&self) -> Self::Column;
}

#[derive(Component)]
pub struct SqlxData {
    pub query: String,
}

#[derive(Default)]
pub struct SqlxPlugin<C: Component>(PhantomData<C>);

impl<C: Debug + Component + SqlxPrimaryKey + Clone + Unpin + for<'r> FromRow<'r, SqliteRow>> Plugin for SqlxPlugin<C> {
    fn build(&self, app: &mut App) {
        let pool = bevy::tasks::block_on(async {
            let env = env::var("DATABASE_URL").unwrap();
            SqlitePool::connect(&env).await.unwrap()
        });
        app.insert_resource(SqlxDatabase { pool });
        app.insert_resource(SqlxTasks::<C>(Vec::new()));
        app.add_event::<SqlxEvent<C>>();
        app.add_systems(Update, (Self::tasks, Self::entities));
    }
}

impl<C: Debug + Component + SqlxPrimaryKey + Clone + Unpin + for<'r> FromRow<'r, SqliteRow>> SqlxPlugin<C> {
    pub fn tasks(
        database: Res<SqlxDatabase>,
        mut tasks: ResMut<SqlxTasks<C>>,
        mut events: EventReader<SqlxEvent<C>>,
    ) {
        for event in events.read() {
            let task_pool = AsyncComputeTaskPool::get();
            let query = event.query.clone();
            let db = database.pool.clone();
            let q = query.clone();
            let task = task_pool.spawn(async move {
                sqlx::query_as(&q).fetch_all(&db).await
            });
            tasks.0.push((query, task));
        }
    }

    pub fn entities(
        world: &mut World,
        params: &mut SystemState<(
            Query<(Entity, &C)>,
            Commands,
            ResMut<SqlxTasks<C>>,
        )>,
    ) {
        let (mut query, mut commands, mut tasks) = params.get_mut(world);

        tasks.0.retain_mut(|(sql, task)| {
            let status = block_on(future::poll_once(task));
            let retain = status.is_none();
            if let Some(result) = status {
                match result {
                    Ok(task_components) => {

                        // TODO: Look into world.spawn_batch after taking set disjunction of ids.

                        for task_component in task_components {
                            let mut spawn = true;

                            // Check if the task's component is already spawned.
                            for (entity, spawned_component) in &mut query {
                                if task_component.id() == spawned_component.id() {
                                    commands.entity(entity)
                                            .remove::<C>()
                                            .insert(task_component.clone())
                                            .insert(SqlxData { query: sql.clone() });
                                    spawn = false;
                                    break;
                                }
                            }

                            if spawn {
                                commands.spawn((
                                    task_component,
                                    SqlxData { query: sql.clone() }
                                ));
                            }
                        }
                    }
                    Err(err) => {
                        dbg!(err);
                    }
                }
            }
            retain
        });

        params.apply(world);
    }
}
