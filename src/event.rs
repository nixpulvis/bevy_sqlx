use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool};
use sqlx::{Database, Error, Executor, IntoArguments, Pool};
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::Arc;
use crate::*;

type SqlxEventFunc<DB, C> = Arc<dyn Fn(Pool<DB>) ->
    Pin<Box<dyn Future<Output = Result<Vec<C>, Error>> + Send>> + Send + Sync>;

/// A [`Event`](bevy::prelude::Event) for fetching data from the [`SqlxDatabase`]
///
/// ### Example
///
/// ```
/// use bevy::prelude::*;
/// use sqlx::{FromRow, Sqlite};
/// use bevy_sqlx::{SqlxPlugin, PrimaryKey, SqlxEvent};
///
/// #[derive(Component, FromRow, Debug)]
/// struct MyRecord {
///     id: u32,
///     flag: bool,
///     text: String,
/// }
///
/// impl PrimaryKey for MyRecord {
///     type Column = u32;
///
///     fn primary_key(&self) -> Self::Column {
///         self.id
///     }
/// }
///
/// let url = "sqlite:db/sqlite.db";
/// App::new()
///     .add_plugins(DefaultPlugins)
///     .add_plugins(SqlxPlugin::<Sqlite, MyRecord>::url(&url))
///     .add_systems(Startup, insert)
///     .add_systems(Update, query)
///     .run();
///
/// fn insert(
///     mut commands: Commands,
///     mut events: EventWriter<SqlxEvent<Sqlite, MyRecord>>,
/// ) {
///     let sql = "INSERT INTO foos(text) VALUES ('test') RETURNING *";
///     SqlxEvent::<Sqlite, MyRecord>::query(sql)
///         .send(&mut events)
///         .trigger(&mut commands);
/// }
///
/// fn query(mut my_records: Query<&MyRecord>) {
///     for my_record in &my_records {
///         dbg!(my_record);
///     }
/// }
/// ```
#[derive(Event, Clone)]
pub struct SqlxEvent<DB: Database, C: SqlxComponent<DB::Row>> {
    label: Option<String>,
    func: SqlxEventFunc<DB, C>,
    _db: PhantomData<DB>,
    _c: PhantomData<C>,
}

impl<DB: Database + Sync, C: SqlxComponent<DB::Row>> SqlxEvent<DB, C>
where
    for<'c> &'c mut DB::Connection: Executor<'c, Database = DB>,
    for<'a> <DB as sqlx::Database>::Arguments<'a>: IntoArguments<'a, DB>,
{
    pub fn query(string: &str) -> Self {
        let arc: Arc<str> = string.into();
        Self::call(Some(string), move |db| {
            let s = arc.clone();
            async move {
                sqlx::query_as(&s).fetch_all(&db).await
            }
        })
    }

    pub fn call<F, T>(label: Option<&str>, func: F) -> Self
    where
        F: Fn(Pool<DB>) -> T + Send + Sync + 'static,
        T: Future<Output = Result<Vec<C>, Error>> + Send + 'static,
    {
        SqlxEvent {
            label: label.map(|s| s.to_string()),
            func: Arc::new(move |db: Pool<DB>| {
                Box::pin(func(db))
            }),
            _db: PhantomData::<DB>,
            _c: PhantomData::<C>,
        }
    }

    pub fn send(self, events: &mut EventWriter<SqlxEvent<DB, C>>) -> Self {
        events.send(SqlxEvent {
            label: self.label.clone(),
            func: self.func.clone(),
            _db: PhantomData::<DB>,
            _c: PhantomData::<C>,
        });
        self
    }

    pub fn trigger(self, commands: &mut Commands) -> Self {
        commands.trigger(SqlxEvent {
            label: self.label.clone(),
            func: self.func.clone(),
            _db: PhantomData::<DB>,
            _c: PhantomData::<C>,
        });
        self
    }

    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    pub(crate) fn func(&self) -> &SqlxEventFunc<DB, C> {
        &self.func
    }
}

impl<DB: Database + Sync, C: SqlxComponent<DB::Row>> SqlxEvent<DB, C>
where
    for<'c> &'c mut <DB as Database>::Connection: Executor<'c, Database = DB>,
    for<'q> <DB as Database>::Arguments<'q>: IntoArguments<'q, DB>,
{
    pub fn handle_events(
        database: Res<SqlxDatabase<DB>>,
        mut tasks: ResMut<SqlxTasks<DB, C>>,
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
}
