use bevy::prelude::*;
use sqlx::{Database, Pool};

/// A [`Resource`](bevy::prelude::Resource) holding a connection to the
/// underlying [`Pool`](sqlx::Pool)
///
/// ### Example
///
/// ```
/// use bevy::prelude::*;
/// use sqlx::Sqlite;
/// use bevy_sqlx::{SqlxPlugin, SqlxDatabase};
/// use bevy_sqlx::component::SqlxDummy;
///
/// let url = "sqlite:db/sqlite.db";
/// App::new()
///     .add_plugins(DefaultPlugins)
///     .add_plugins(SqlxPlugin::<Sqlite, SqlxDummy>::url(&url))
///     .add_systems(Startup, resource)
///     .run();
///
/// fn resource(db: Res<SqlxDatabase<Sqlite>>) {
///     let record = bevy::tasks::block_on(async {
///         sqlx::query!("SELECT (1) as id, 'test' as text")
///             .fetch_one(&db.pool)
///             .await.unwrap()
///     });
///
///     dbg!(record);
/// }
/// ```
#[derive(Resource, Debug)]
pub struct SqlxDatabase<DB: Database> {
    pub pool: Pool<DB>,
}
