use crate::*;
use bevy::prelude::*;
use bevy::tasks::futures_lite::future;
use bevy::tasks::{block_on, Task};
use sqlx::{Database, Error, Executor, IntoArguments};
use std::marker::PhantomData;

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
    pub(crate) components:
        Vec<(SqlxEventId, bool, Task<Result<Vec<C>, Error>>)>,
    _r: PhantomData<DB::Row>,
}

impl<DB: Database, C: SqlxComponent<DB::Row>> Default for SqlxTasks<DB, C> {
    fn default() -> Self {
        SqlxTasks { components: Vec::new(), _r: PhantomData::<DB::Row> }
    }
}

impl<DB: Database + Sync, C: SqlxComponent<DB::Row>> SqlxTasks<DB, C>
where
    for<'c> &'c mut <DB as Database>::Connection: Executor<'c, Database = DB>,
    for<'q> <DB as Database>::Arguments<'q>: IntoArguments<'q, DB>,
{
    /// An exclusive [`System`] which polls [`Task`]s for `Result<C, Error>`
    ///
    /// Tasks are spawned in [`SqlxEvent::handle_events`].
    ///
    /// If [`SqlxEvent::will_sync`] was `true`:
    ///
    /// For each component returned, we look up its primary key in
    /// [`SqlxIndex<C>`]:
    /// - If an entity is indexed under that key, we `insert` the new component
    /// over the existing one and send an [`SqlxEventStatus::Update`]
    /// - If not, we `spawn` a new entity with the new component and send an
    /// [`SqlxEventStatus::Spawn`]
    ///
    /// Either way the entity is recorded in the index straight away, so a row
    /// returned again later in the same frame updates it instead of spawning
    /// a duplicate.
    ///
    /// If [`SqlxEvent::will_sync`] was `false`:
    ///
    /// - We send an [`SqlxEventStatus::Return`] with the component itself.
    pub fn handle_tasks(world: &mut World) {
        let mut finished = Vec::new();
        world.resource_mut::<Self>().components.retain_mut(
            |(id, sync, task)| match block_on(future::poll_once(task)) {
                Some(result) => {
                    finished.push((*id, *sync, result));
                    false
                }
                None => true,
            },
        );

        for (id, sync, result) in finished {
            match result {
                Ok(task_components) if sync => {
                    for task_component in task_components {
                        Self::sync_component(world, id, task_component);
                    }
                }
                Ok(task_components) => {
                    world.send_event(SqlxEventStatus::<DB, C>::Return(
                        id,
                        task_components,
                    ));
                }
                Err(err) => {
                    warn!("sqlx event {id} failed: {err}");
                    world.send_event(SqlxEventStatus::<DB, C>::Error(id, err));
                }
            }
        }
    }

    /// Spawn or update the entity for `component`'s row, and index it
    fn sync_component(world: &mut World, id: SqlxEventId, component: C) {
        let key = component.primary_key();
        let existing = world
            .resource::<SqlxIndex<C>>()
            .entity(&key)
            .filter(|entity| world.get_entity(*entity).is_some());

        let (entity, status) = match existing {
            Some(entity) => {
                world.entity_mut(entity).insert(component);
                (entity, SqlxEventStatus::Update(id, key.clone(), PhantomData))
            }
            None => {
                let entity = world.spawn(component).id();
                (entity, SqlxEventStatus::Spawn(id, key.clone(), PhantomData))
            }
        };

        let tick = world.entity(entity).get_ref::<C>().unwrap().last_changed();
        world.resource_mut::<SqlxIndex<C>>().insert(key, entity, tick);
        world.send_event::<SqlxEventStatus<DB, C>>(status);
    }

    pub fn count(&self) -> usize {
        self.components.len()
    }

    pub fn is_empty(&self) -> bool {
        self.components.is_empty()
    }
}
