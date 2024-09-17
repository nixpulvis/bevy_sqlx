use bevy::ecs::system::SystemState;
use bevy::prelude::*;
use bevy::tasks::futures_lite::future;
use bevy::tasks::{block_on, AsyncComputeTaskPool};
use sqlx::{Database, Executor, IntoArguments, Pool};
use std::marker::PhantomData;
use crate::*;

pub struct SqlxPlugin<DB: Database, C: SqlxComponent<DB::Row>> {
    pool: Pool<DB>,
    _c: PhantomData<C>,
}

impl<DB: Database, C: SqlxComponent<DB::Row>> SqlxPlugin<DB, C> {
    pub fn pool(pool: Pool<DB>) -> Self {
        SqlxPlugin {
            pool,
            _c: PhantomData,
        }
    }

    pub fn url(url: &str) -> Self {
        let pool = bevy::tasks::block_on(async { Pool::connect(url).await.unwrap() });
        SqlxPlugin {
            pool,
            _c: PhantomData,
        }
    }
}

impl<DB: Database + Sync, C: SqlxComponent<DB::Row>> Plugin for SqlxPlugin<DB, C>
where
    for<'c> &'c mut <DB as Database>::Connection: Executor<'c, Database = DB>,
    for<'q> <DB as Database>::Arguments<'q>: IntoArguments<'q, DB>,
{
    fn build(&self, app: &mut App) {
        app.insert_resource(SqlxDatabase {
            pool: self.pool.clone(),
        });
        app.insert_resource(SqlxTasks::<DB::Row, C>::default());
        app.add_event::<SqlxEvent<DB, C>>();
        app.add_systems(Update, (Self::tasks, Self::entities));
    }
}

impl<DB: Database + Sync, C: SqlxComponent<DB::Row>> SqlxPlugin<DB, C>
where
    for<'c> &'c mut <DB as Database>::Connection: Executor<'c, Database = DB>,
    for<'q> <DB as Database>::Arguments<'q>: IntoArguments<'q, DB>,
{
    pub fn tasks(
        database: Res<SqlxDatabase<DB>>,
        mut tasks: ResMut<SqlxTasks<DB::Row, C>>,
        mut events: EventReader<SqlxEvent<DB, C>>,
    ) {
        let task_pool = AsyncComputeTaskPool::get();
        for event in events.read() {
            let db = database.pool.clone();
            let future = (event.func())(db);
            let task = task_pool.spawn(async move { future.await });
            tasks.components.push(task);
        }
    }

    pub fn entities(
        world: &mut World,
        params: &mut SystemState<(
            Query<(Entity, Ref<C>)>,
            Commands,
            ResMut<SqlxTasks<DB::Row, C>>,
        )>,
    ) {
        let (mut query, mut commands, mut tasks) = params.get_mut(world);

        // for (entity, component) in &mut query {
        //     // TODO: Send Encoded UPDATE or callback function?
        //     // TODO: Need a dirty bit to check so we don't send just
        //     //       received updated entities.
        //     if component.is_changed() && !component.is_added() {
        //         dbg!("TODO: UPDATE");
        //     }
        // }

        tasks.components.retain_mut(|task| {
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
                                commands.entity(entity).insert(task_component);
                            } else {
                                commands.spawn(task_component);
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
