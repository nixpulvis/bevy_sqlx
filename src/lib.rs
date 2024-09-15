#![feature(trivial_bounds)]
use std::env;
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

pub trait SqlxComponent:
    SqlxPrimaryKey +
    Component +
    for<'r> FromRow<'r, SqliteRow> +
    Unpin
{}
impl<C> SqlxComponent for C
where
    C: SqlxPrimaryKey +
        Component +
        for<'r> FromRow<'r, SqliteRow> +
        Unpin
{}

#[derive(Resource, Debug)]
pub struct SqlxDatabase {
    pub pool: SqlitePool
}

#[derive(Resource, Debug)]
pub struct SqlxTasks<C: SqlxComponent>(pub Vec<(String, Task<Result<Vec<C>, Error>>)>);


#[derive(Event, Debug)]
pub struct SqlxEvent<C: SqlxComponent> {
    pub query: String,
    _c: PhantomData<C>,
}

impl<C: SqlxComponent> SqlxEvent<C> {
    pub fn query(string: &str) -> Self {
        SqlxEvent {
            query: string.to_string(),
            _c: PhantomData::<C>,
        }
    }

    pub fn send(self, events: &mut EventWriter<SqlxEvent<C>>) -> Self {
        events.send(SqlxEvent {
            query: self.query.clone(),
            _c: PhantomData::<C>,
        });
        self
    }

    pub fn trigger(self, commands: &mut Commands) -> Self {
        commands.trigger(SqlxEvent {
            query: self.query.clone(),
            _c: PhantomData::<C>,
        });
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

#[derive(Reflect, Component, Debug)]
pub struct SqlxData {
    pub query: String,
}

pub struct SqlxPlugin<C: SqlxComponent> {
    url: Option<String>,
    _c: PhantomData<C>,
}

impl<C: SqlxComponent> Default for SqlxPlugin<C> {
    fn default() -> Self {
        SqlxPlugin {
            url: None,
            _c: PhantomData,
        }
    }
}

impl<C: SqlxComponent> SqlxPlugin<C> {
    pub fn url(string: &str) -> Self {
        SqlxPlugin {
            url: Some(string.to_string()),
            _c: PhantomData,
        }
    }
}

impl<C: SqlxComponent> Plugin for SqlxPlugin<C> {
    fn build(&self, app: &mut App) {
        let pool = bevy::tasks::block_on(async {
            let url = self.url.clone()
                .unwrap_or(env::var("DATABASE_URL").unwrap());
            SqlitePool::connect(&url).await.unwrap()
        });
        app.insert_resource(SqlxDatabase { pool });
        app.insert_resource(SqlxTasks::<C>(Vec::new()));
        app.add_event::<SqlxEvent<C>>();
        app.register_type::<SqlxData>();
        app.add_systems(Update, (Self::tasks, Self::entities));
    }
}

impl<C: SqlxComponent> SqlxPlugin<C> {
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
            Query<(Entity, Ref<C>)>,
            Commands,
            ResMut<SqlxTasks<C>>,
        )>,
    ) {
        let (mut query, mut commands, mut tasks) = params.get_mut(world);

        // for (entity, component) in &mut query {
        //     // TODO: Send Encoded UPDATE
        //     // TODO: Need a dirty bit to check so we don't send just
        //     //       received updated entities.
        //     if component.is_changed() && !component.is_added() {
        //         dbg!("TODO: UPDATE");
        //     }
        // }

        tasks.0.retain_mut(|(sql, task)| {
            let status = block_on(future::poll_once(task));
            let retain = status.is_none();
            if let Some(result) = status {
                match result {
                    Ok(task_components) => {
                        // TODO: Look into world.spawn_batch after taking set disjunction of ids.
                        for task_component in task_components {
                            // Check if the task's component is already spawned.
                            let mut existing_entity = None;
                            for (entity, spawned_component) in &mut query {
                                if task_component.id() == spawned_component.id() {
                                    existing_entity = Some(entity);
                                    break;
                                }
                            }

                            if let Some(entity) = existing_entity {
                                commands.entity(entity)
                                        .insert(task_component)
                                        .insert(SqlxData { query: sql.clone() });
                            } else {
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
