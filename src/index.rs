use crate::*;
use bevy::ecs::component::Tick;
use bevy::ecs::entity::EntityHashMap;
use bevy::prelude::*;
use bevy::utils::HashMap;

/// A [`Resource`](bevy::prelude::Resource) mapping primary keys to the
/// entities synced from their rows, in both directions
///
/// [`SqlxTasks::handle_tasks`] records an entity here when it spawns or
/// updates it from a row, so later rows with the same key update that entity
/// instead of spawning another. Entries are removed when the entity is
/// despawned or its `C` is removed.
///
/// An entity with a `C` that isn't in the index wasn't synced from the
/// database (for example, game code spawned it directly).
///
/// ### Example
///
/// ```
/// # use bevy::prelude::*;
/// # use sqlx::FromRow;
/// # use bevy_sqlx::{PrimaryKey, SqlxIndex};
/// # #[derive(Component, FromRow, Debug)]
/// # struct Foo { id: u32 }
/// # impl PrimaryKey for Foo {
/// #     type Key = u32;
/// #     fn primary_key(&self) -> Self::Key { self.id }
/// # }
/// fn lookup(index: Res<SqlxIndex<Foo>>, foos: Query<&Foo>) {
///     if let Some(entity) = index.entity(&1) {
///         dbg!(foos.get(entity));
///     }
/// }
/// ```
#[derive(Resource, Debug)]
pub struct SqlxIndex<C: PrimaryKey> {
    by_key: HashMap<C::Key, Entity>,
    by_entity: EntityHashMap<C::Key>,
    synced_at: EntityHashMap<Tick>,
}

impl<C: PrimaryKey> Default for SqlxIndex<C> {
    fn default() -> Self {
        SqlxIndex {
            by_key: HashMap::default(),
            by_entity: EntityHashMap::default(),
            synced_at: EntityHashMap::default(),
        }
    }
}

impl<C: PrimaryKey> SqlxIndex<C> {
    /// The entity synced from the row with this primary key
    pub fn entity(&self, key: &C::Key) -> Option<Entity> {
        self.by_key.get(key).copied()
    }

    /// The primary key of the row this entity was synced from
    pub fn key(&self, entity: Entity) -> Option<&C::Key> {
        self.by_entity.get(&entity)
    }

    /// The change tick at which this entity's `C` was last written from the
    /// database
    pub fn synced_at(&self, entity: Entity) -> Option<Tick> {
        self.synced_at.get(&entity).copied()
    }

    /// Iterate over every `(key, entity)` pair
    pub fn iter(&self) -> impl Iterator<Item = (&C::Key, Entity)> {
        self.by_key.iter().map(|(key, entity)| (key, *entity))
    }

    pub fn len(&self) -> usize {
        self.by_key.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_key.is_empty()
    }

    /// Record that `entity` was synced from the row with `key` at `tick`
    pub(crate) fn insert(&mut self, key: C::Key, entity: Entity, tick: Tick) {
        if let Some(old_key) = self.by_entity.insert(entity, key.clone()) {
            if old_key != key {
                self.by_key.remove(&old_key);
            }
        }
        if let Some(old_entity) = self.by_key.insert(key, entity) {
            if old_entity != entity {
                self.by_entity.remove(&old_entity);
                self.synced_at.remove(&old_entity);
            }
        }
        self.synced_at.insert(entity, tick);
    }

    /// Forget `entity`, returning the key it was indexed under
    pub(crate) fn remove(&mut self, entity: Entity) -> Option<C::Key> {
        self.synced_at.remove(&entity);
        let key = self.by_entity.remove(&entity)?;
        self.by_key.remove(&key);
        Some(key)
    }
}

impl<C: PrimaryKey + Component> SqlxIndex<C> {
    /// An observer which keeps the index current when `C` is removed or its
    /// entity is despawned
    ///
    /// This is an observer rather than a component hook because a component
    /// can only have one hook, and that belongs to the user.
    pub fn on_remove(trigger: Trigger<OnRemove, C>, mut index: ResMut<Self>) {
        index.remove(trigger.entity());
    }
}
