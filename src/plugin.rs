use crate::*;
use bevy::prelude::*;
use bevy::tasks::block_on;
use sqlx::{Database, Executor, IntoArguments, Pool};
use std::marker::PhantomData;

/// A [`Plugin`](bevy::prelude::Plugin) to add to an
/// [`App`](bevy::prelude::App)
///
/// This plugin sets up and manages the following:
/// - A [`SqlxDatabase<DB>`] resource
/// - A [`SqlxTasks<DB::Row, C>`] resource
/// - [`SqlxEvent<DB, C>`] events
/// - A [`SqlxEvent<DB, C>::handle_events`] system
/// - A [`SqlxTasks<DB, C>::handle_tasks`] system
//
// TODO: test multiple of these at once
pub struct SqlxPlugin<DB: Database, C: SqlxComponent<DB::Row>> {
    pool: Pool<DB>,
    _c: PhantomData<C>,
}

impl<DB: Database, C: SqlxComponent<DB::Row>> SqlxPlugin<DB, C> {
    /// Build a new plugin directly from the given pool
    ///
    /// ```
    /// use bevy::tasks::block_on;
    /// use sqlx::{Pool, Sqlite};
    /// use bevy_sqlx::{SqlxPlugin, SqlxDummy};
    ///
    /// let url = "sqlite:db/sqlite.db";
    /// let pool = block_on(async {
    ///     Pool::connect(url).await.unwrap()
    /// });
    /// SqlxPlugin::<Sqlite, SqlxDummy>::from_pool(pool);
    /// ```
    pub fn from_pool(pool: Pool<DB>) -> Self {
        SqlxPlugin { pool, _c: PhantomData }
    }

    /// Build a plugin with a new connection from the given `url`
    ///
    /// ```
    /// use sqlx::Sqlite;
    /// use bevy_sqlx::{SqlxPlugin, SqlxDummy};
    ///
    /// SqlxPlugin::<Sqlite, SqlxDummy>::from_url("sqlite:db/sqlite.db");
    /// ```
    pub fn from_url(url: &str) -> Self {
        let pool = block_on(async { Pool::connect(url).await.unwrap() });
        SqlxPlugin { pool, _c: PhantomData }
    }
}

impl<DB: Database + Sync, C: SqlxComponent<DB::Row>> Plugin
    for SqlxPlugin<DB, C>
where
    for<'c> &'c mut <DB as Database>::Connection: Executor<'c, Database = DB>,
    for<'q> <DB as Database>::Arguments<'q>: IntoArguments<'q, DB>,
{
    fn build(&self, app: &mut App) {
        app.insert_resource(SqlxDatabase { pool: self.pool.clone() });
        app.insert_resource(SqlxTasks::<DB, C>::default());
        app.add_event::<SqlxEvent<DB, C>>();
        app.add_event::<SqlxEventStatus<DB, C>>();
        app.add_systems(Update, SqlxEvent::<DB, C>::handle_events);
        app.add_systems(Update, SqlxTasks::<DB, C>::handle_tasks);
    }
}
