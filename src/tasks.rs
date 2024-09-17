use bevy::ecs::system::SystemState;
use bevy::prelude::*;
use bevy::tasks::futures_lite::future;
use bevy::tasks::{block_on, Task};
use sqlx::{Database, Executor, IntoArguments, Error};
use std::marker::PhantomData;
use crate::*;

/// A [`Resource`](bevy::prelude::Resource) of tasks with the resulting
/// components from the database
///
/// ### Example
///
/// ```
/// # use bevy::prelude::*;
/// # #[derive(Component, Debug)]
/// # struct Foo;
/// fn query(mut foos: Query<&Foo>) {
///     for foo in &foos {
///         dbg!(foo);
///     }
/// }
/// ```
#[derive(Resource, Debug)]
pub struct SqlxTasks<DB: Database, C: SqlxComponent<DB::Row>> {
    pub components: Vec<(Option<String>, Task<Result<Vec<C>, Error>>)>,
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
    /// An exclusive [`System`] which polls [`Task`]s in [`ResMut<SqlxTasks<DB,
    /// C>>`]
    ///
    /// Tasks are spawned in [`SqlxEvent::handle_events`].
    ///
    /// When a task is finished, we check if the component is already spawned:
    /// - If it is, we just `insert` the new component over the existing one
    /// - If it isn't, we `spawn` a new entity with the new component
    pub fn handle_tasks(
        world: &mut World,
        params: &mut SystemState<(
            Query<(Entity, Ref<C>)>,
            Commands,
            ResMut<Self>,
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

        tasks.components.retain_mut(|(label, task)| {
            block_on(future::poll_once(task)).map(|result| {
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
                                    Update(
                                        label.clone(),
                                        task_component.primary_key(),
                                        PhantomData));
                                commands.entity(entity).insert(task_component);
                            } else {
                                status.send(SqlxEventStatus::
                                    Spawn(
                                        label.clone(),
                                        task_component.primary_key(),
                                        PhantomData));
                                commands.spawn(task_component);
                            }
                        }
                    }
                    Err(err) => {
                        status.send(SqlxEventStatus::
                            Error(label.clone(), err));
                    }
                }
            }).is_none()
        });

        params.apply(world);
    }
}
