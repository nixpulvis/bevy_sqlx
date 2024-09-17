use bevy::ecs::system::SystemState;
use bevy::prelude::*;
use bevy::tasks::futures_lite::future;
use bevy::tasks::{block_on, Task};
use sqlx::{Database, Executor, IntoArguments, Error};
use std::marker::PhantomData;
use crate::*;

/// A [`Resource`](bevy::prelude::Resource) of tasks with the resulting
/// components from the database
#[derive(Resource, Debug)]
pub struct SqlxTasks<DB: Database, C: SqlxComponent<DB::Row>> {
    pub components: Vec<Task<Result<Vec<C>, Error>>>,
    _r: PhantomData<DB::Row>,
}

impl<DB: Database, C: SqlxComponent<DB::Row>> Default for SqlxTasks<DB, C> {
    fn default() -> Self {
        SqlxTasks {
            components: Vec::new(),
            _r: PhantomData::<DB::Row>,
        }
    }
}

impl<DB: Database + Sync, C: SqlxComponent<DB::Row>> SqlxTasks<DB, C>
where
    for<'c> &'c mut <DB as Database>::Connection: Executor<'c, Database = DB>,
    for<'q> <DB as Database>::Arguments<'q>: IntoArguments<'q, DB>,
{
    pub fn handle_tasks(
        world: &mut World,
        params: &mut SystemState<(
            Query<(Entity, Ref<C>)>,
            Commands,
            ResMut<SqlxTasks<DB, C>>,
            EventWriter<SqlxEventStatus<DB, C>>,
        )>,
    ) {
        let (mut query, mut commands, mut tasks, mut status) =
            params.get_mut(world);

        // for (entity, component) in &mut query {
        //     // TODO: Send Encoded UPDATE or callback function?
        //     // TODO: Need a dirty bit to check so we don't send just
        //     //       received updated entities.
        //     if component.is_changed() && !component.is_added() {
        //         dbg!("TODO: UPDATE");
        //     }
        // }

        tasks.components.retain_mut(|task| {
            let poll = block_on(future::poll_once(task));
            // TODO refactor to remove this variable
            let retain = poll.is_none();
            if let Some(result) = poll {
                match result {
                    Ok(task_components) => {
                        // TODO: Look into world.spawn_batch after taking set
                        // disjunction of ids.
                        for task_component in task_components {

                            // Check if the task's component is already spawned.
                            let mut existing_entity = None;
                            for (entity, spawned_component) in &mut query {
                                if task_component.primary_key() ==
                                   spawned_component.primary_key()
                                {
                                    existing_entity = Some(entity);
                                    break;
                                }
                            }

                            if let Some(entity) = existing_entity {
                                status.send(SqlxEventStatus::
                                    Insert(task_component.primary_key(),
                                            PhantomData));
                                commands.entity(entity).insert(task_component);
                            } else {
                                status.send(SqlxEventStatus::
                                    Spawn(task_component.primary_key(),
                                            PhantomData));
                                commands.spawn(task_component);
                            }
                        }
                    }
                    Err(err) => {
                        status.send(SqlxEventStatus::Error(err));
                    }
                }
            }
            retain
        });

        params.apply(world);
    }
}
