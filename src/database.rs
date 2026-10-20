use bevy::prelude::*;
use sqlx::{Database, Pool};

/// A [`Resource`](bevy::prelude::Resource) holding a connection to the
/// underlying [`Pool`](sqlx::Pool)
///
/// ### Example
///
/// ```no_run
/// use bevy::prelude::*;
/// use sqlx::Sqlite;
/// use bevy_sqlx::{SqlxPlugin, SqlxDatabase};
/// use bevy_sqlx::component::SqlxDummy;
///
/// let url = "sqlite:db/sqlite.db";
/// App::new()
///     .add_plugins(DefaultPlugins)
///     .add_plugins(SqlxPlugin::<Sqlite, SqlxDummy>::from_url(&url))
///     .add_systems(Startup, resource)
///     .run();
///
/// fn resource(db: Res<SqlxDatabase<Sqlite>>) {
///     let record = bevy::tasks::block_on(async {
///         sqlx::query_as::<_, (i64, String)>("SELECT 1 AS id, 'test' AS text")
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
