//! Both writer [`SqlxEvent`] and reader [`SqlxEventStatus`]
//!
//! Sending a single [`SqlxEvent`] will start by sending it's own:
//! - [`SqlxEventStatus::Start`]
//!
//! Then, depending on how the event's task in [`SqlxTasks`] is
//! processed, one of:
//! - [`SqlxEventStatus::Spawn`]
//! - [`SqlxEventStatus::Update`]
use crate::*;
use bevy::prelude::*;
use bevy::tasks::AsyncComputeTaskPool;
use sqlx::{Database, Error, Executor, IntoArguments, Pool};
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

/// The type of [`SqlxEvent`] IDs
pub type SqlxEventId = u32;

pub(crate) static EVENT_ID_GENERATOR: AtomicU32 = AtomicU32::new(1);

// TODO: Take note of when this should be expected to overflow and if it's
// worth the cost in generating unique IDs
pub fn next_event_id() -> SqlxEventId {
    EVENT_ID_GENERATOR.fetch_add(1, Ordering::Relaxed)
}

/// An [`Event`] for fetching data from the [`SqlxDatabase`]
///
/// When a [`SqlxPlugin`] is added to an app, [`SqlxEvent::handle_events`] is
/// added too.
///
/// ### Example
///
/// ```
/// # use bevy::prelude::*;
/// # use sqlx::{FromRow, Sqlite};
/// # use bevy_sqlx::{SqlxPlugin, PrimaryKey, SqlxEvent};
/// # #[derive(Component, FromRow, Debug)]
/// # struct Foo(u32);
/// # impl PrimaryKey for Foo {
/// #     type Column = u32;
/// #     fn primary_key(&self) -> Self::Column { self.0 }
/// # }
/// fn insert(mut events: EventWriter<SqlxEvent<Sqlite, Foo>>) {
///     let sql = "INSERT INTO foos(text) VALUES ('test') RETURNING *";
///     events.send(SqlxEvent::<Sqlite, Foo>::query_sync(sql));
/// }
/// ```
#[derive(Event, Clone)]
pub struct SqlxEvent<DB: Database, C: SqlxComponent<DB::Row>> {
    pub(crate) func: SqlxEventFunc<DB, C>,
    id: SqlxEventId,
    will_sync: bool,
    _db: PhantomData<DB>,
    _c: PhantomData<C>,
}

type SqlxEventFunc<DB, C> = Arc<
    dyn Fn(
            Pool<DB>,
        )
            -> Pin<Box<dyn Future<Output = Result<Vec<C>, Error>> + Send>>
        + Send
        + Sync,
>;

impl<DB: Database + Sync, C: SqlxComponent<DB::Row>> SqlxEvent<DB, C>
where
    for<'c> &'c mut DB::Connection: Executor<'c, Database = DB>,
    for<'a> <DB as sqlx::Database>::Arguments<'a>: IntoArguments<'a, DB>,
{
    /// Construct a new [`SqlxEvent`] from the given SQL string
    ///
    /// See [`Self::call`] for information.
    /// ```
    /// use sqlx::Sqlite;
    /// use bevy_sqlx::{SqlxEvent, SqlxDummy};
    ///
    /// SqlxEvent::<Sqlite, SqlxDummy>::query("SELECT * FROM foos");
    /// ```
    pub fn query(sql: &str) -> Self {
        Self::query_private(false, sql)
    }

    /// Construct a new synchronizing [`SqlxEvent`] from the given SQL string
    ///
    /// See [`Self::call_sync`] for more information.
    pub fn query_sync(sql: &str) -> Self {
        Self::query_private(true, sql)
    }

    fn query_private(sync: bool, sql: &str) -> Self {
        let arc: Arc<str> = sql.into();
        Self::call_private(sync, move |db| {
            let s = arc.clone();
            async move { sqlx::query_as(&s).fetch_all(&db).await }
        })
    }

    /// Construct a new [`SqlxEvent`] from the given function with access
    /// to a [`Pool<DB>`]
    ///
    /// Upon a successful DB interaction, a [`SqlxEventStatus::Return`] event
    /// will be sent.
    ///
    /// ```
    /// use sqlx::Sqlite;
    /// use bevy_sqlx::{SqlxEvent, SqlxDummy};
    ///
    /// SqlxEvent::<Sqlite, SqlxDummy>::call(move |db| { async move {
    ///     sqlx::query_as("INSERT INTO foos (text) VALUES (?) RETURNING *")
    ///         .bind("hello")
    ///         .fetch_all(&db).await
    /// }});
    /// ```
    pub fn call<F, T>(func: F) -> Self
    where
        F: Fn(Pool<DB>) -> T + Send + Sync + 'static,
        T: Future<Output = Result<Vec<C>, Error>> + Send + 'static,
    {
        Self::call_private(false, func)
    }

    /// Construct a new synchronizing [`SqlxEvent`] from the given function
    /// with access to a [`Pool<DB>`]
    ///
    /// Upon a successful DB interaction, either of the following events will
    /// be sent:
    ///
    /// - [`SqlxEventStatus::Spawn`]
    /// - [`SqlxEventStatus::Update`]
    ///
    /// See [`Self::call`] for more information.
    pub fn call_sync<F, T>(func: F) -> Self
    where
        F: Fn(Pool<DB>) -> T + Send + Sync + 'static,
        T: Future<Output = Result<Vec<C>, Error>> + Send + 'static,
    {
        Self::call_private(true, func)
    }

    fn call_private<F, T>(sync: bool, func: F) -> Self
    where
        F: Fn(Pool<DB>) -> T + Send + Sync + 'static,
        T: Future<Output = Result<Vec<C>, Error>> + Send + 'static,
    {
        SqlxEvent {
            func: Arc::new(move |db: Pool<DB>| Box::pin(func(db))),
            id: next_event_id(),
            will_sync: sync,
            _db: PhantomData::<DB>,
            _c: PhantomData::<C>,
        }
    }

    /// Return the id of this event
    pub fn id(&self) -> SqlxEventId {
        self.id
    }

    /// Return true if this event will sync its component to the ECS
    pub fn will_sync(&self) -> bool {
        self.will_sync
    }
}

/// An [`Event`] sent while processing an [`SqlxEvent`]
///
/// ### Example
///
/// ```
/// # use bevy::prelude::*;
/// # use sqlx::Sqlite;
/// # use bevy_sqlx::{SqlxEventStatus, SqlxDummy};
/// fn status(mut statuses: EventReader<SqlxEventStatus<Sqlite, SqlxDummy>>) {
///     for status in statuses.read() {
///         match status {
///             SqlxEventStatus::Start(id) => {},
///             SqlxEventStatus::Return(id, comp) => {},
///             SqlxEventStatus::Spawn(id, pk, _) => {},
///             SqlxEventStatus::Update(id, pk, _) => {},
///             SqlxEventStatus::Error(id, err) => {},
///         }
///     }
/// }
/// ```
#[derive(Event, Debug)]
pub enum SqlxEventStatus<DB: Database, C: SqlxComponent<DB::Row>> {
    Start(SqlxEventId),
    Return(SqlxEventId, Vec<C>),
    Spawn(SqlxEventId, C::Column, PhantomData<DB>),
    Update(SqlxEventId, C::Column, PhantomData<DB>),
    Error(SqlxEventId, Error),
}

impl<DB: Database, C: SqlxComponent<DB::Row>> SqlxEventStatus<DB, C> {
    pub fn id(&self) -> SqlxEventId {
        match *self {
            SqlxEventStatus::Start(id)
            | SqlxEventStatus::Return(id, _)
            | SqlxEventStatus::Spawn(id, _, _)
            | SqlxEventStatus::Update(id, _, _)
            | SqlxEventStatus::Error(id, _) => id,
        }
    }
}

impl<DB: Database + Sync, C: SqlxComponent<DB::Row>> SqlxEvent<DB, C>
where
    for<'c> &'c mut <DB as Database>::Connection: Executor<'c, Database = DB>,
    for<'q> <DB as Database>::Arguments<'q>: IntoArguments<'q, DB>,
{
    /// A [`System`] which listens for [`SqlxEvent`]s and processes them
    ///
    /// This system performs the following actions:
    /// - A [`SqlxEventStatus::Start`] event is sent
    /// - A new [`Task`](bevy::tasks::Task) for [`SqlxTasks::handle_tasks`]
    /// is spawned
    pub fn handle_events(
        database: Res<SqlxDatabase<DB>>,
        mut tasks: ResMut<SqlxTasks<DB, C>>,
        mut events: EventReader<SqlxEvent<DB, C>>,
        mut status: EventWriter<SqlxEventStatus<DB, C>>,
    ) {
        let task_pool = AsyncComputeTaskPool::get();
        for event in events.read() {
            status.send(SqlxEventStatus::Start(event.id()));
            let db = database.pool.clone();
            let future = (event.func)(db);
            let task = task_pool.spawn(async move { future.await });
            tasks.components.push((event.id(), event.will_sync(), task));
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::*;
    use assert_matches::assert_matches;
    use bevy::ecs::system::SystemState;
    use bevy::prelude::*;
    use bevy::tasks::{block_on, AsyncComputeTaskPool, TaskPool};
    use sqlx::sqlite::SqlitePoolOptions;
    use sqlx::{FromRow, Sqlite};
    use std::time::{Duration, Instant};

    #[derive(Component, FromRow, Debug)]
    struct Foo {
        id: u32,
        text: String,
    }

    impl PrimaryKey for Foo {
        type Column = u32;
        fn primary_key(&self) -> Self::Column {
            self.id
        }
    }

    /// A fresh in-memory database per test, so tests can run in parallel
    /// without sharing state or contending for SQLite's write lock.
    fn setup_app() -> App {
        AsyncComputeTaskPool::get_or_init(|| TaskPool::new());
        let pool = block_on(async {
            // Every connection to `sqlite::memory:` is its own database, so
            // keep exactly one connection open for the life of the pool.
            let pool = SqlitePoolOptions::new()
                .max_connections(1)
                .idle_timeout(None)
                .max_lifetime(None)
                .connect("sqlite::memory:")
                .await
                .unwrap();
            sqlx::migrate!().run(&pool).await.unwrap();
            pool
        });
        let mut app = App::new();
        app.add_plugins(SqlxPlugin::<Sqlite, Foo>::from_pool(pool));
        app
    }

    /// Update the app until the event finishes, then check its status
    ///
    /// Panics on an [`SqlxEventStatus::Error`] or if nothing finishes within
    /// a few seconds, rather than letting a test spin on a failed query.
    fn wait_for_status(
        app: &mut App,
        system_state: &mut SystemState<
            EventReader<SqlxEventStatus<Sqlite, Foo>>,
        >,
        check: impl Fn(&SqlxEventStatus<Sqlite, Foo>),
    ) {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            app.update();
            let mut reader = system_state.get(app.world());
            for status in reader.read() {
                match status {
                    SqlxEventStatus::Start(_) => {}
                    SqlxEventStatus::Error(_, err) => {
                        panic!("query failed: {err}")
                    }
                    status => return check(status),
                }
            }
            assert!(Instant::now() < deadline, "timed out waiting for status");
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    #[test]
    fn test_query() {
        let mut app = setup_app();
        let mut system_state: SystemState<
            EventReader<SqlxEventStatus<Sqlite, Foo>>,
        > = SystemState::new(app.world_mut());

        let sql = "INSERT INTO foos (text) VALUES ('query') RETURNING *";
        let insert = SqlxEvent::<Sqlite, Foo>::query(sql);
        app.world_mut().send_event(insert);

        wait_for_status(&mut app, &mut system_state, |status| {
            assert_matches!(status,
                SqlxEventStatus::Return(_, components)
                    if components[0].text == "query");
        });
    }

    #[test]
    fn test_query_sync() {
        let mut app = setup_app();
        let mut system_state: SystemState<
            EventReader<SqlxEventStatus<Sqlite, Foo>>,
        > = SystemState::new(app.world_mut());
        let mut query_state: SystemState<Query<&Foo>> =
            SystemState::new(app.world_mut());

        let sql = "INSERT INTO foos (text) VALUES ('query_sync') RETURNING *";
        let insert = SqlxEvent::<Sqlite, Foo>::query_sync(sql);
        app.world_mut().send_event(insert);

        wait_for_status(&mut app, &mut system_state, |_| {});

        let query = query_state.get(app.world());
        assert_eq!("query_sync", query.single().text);
    }

    #[test]
    fn test_call_sync() {
        let mut app = setup_app();
        let mut system_state: SystemState<
            EventReader<SqlxEventStatus<Sqlite, Foo>>,
        > = SystemState::new(app.world_mut());
        let mut query_state: SystemState<Query<&Foo>> =
            SystemState::new(app.world_mut());

        let text = "call_sync";
        let insert =
            SqlxEvent::<Sqlite, Foo>::call_sync(move |db| async move {
                sqlx::query_as("INSERT INTO foos (text) VALUES (?) RETURNING *")
                    .bind(text)
                    .fetch_all(&db)
                    .await
            });
        app.world_mut().send_event(insert);

        wait_for_status(&mut app, &mut system_state, |_| {});

        let query = query_state.get(app.world());
        assert_eq!(text, query.single().text);
    }

    #[test]
    fn test_event_status_started() {
        let mut app = setup_app();
        let mut system_state: SystemState<
            EventReader<SqlxEventStatus<Sqlite, Foo>>,
        > = SystemState::new(app.world_mut());

        let sql = "INSERT INTO foos (text) VALUES ('spawn') RETURNING *";
        let insert = SqlxEvent::<Sqlite, Foo>::query_sync(sql);
        app.world_mut().send_event(insert);

        let mut reader = system_state.get(app.world());
        let events = reader.read();
        assert_eq!(0, events.len());

        app.update();

        let mut reader = system_state.get(app.world());
        let mut events = reader.read();
        assert_matches!(events.next().unwrap(), SqlxEventStatus::Start(_));
    }

    #[test]
    fn test_event_status_spawn() {
        let mut app = setup_app();
        let mut system_state: SystemState<
            EventReader<SqlxEventStatus<Sqlite, Foo>>,
        > = SystemState::new(app.world_mut());

        let sql = "INSERT INTO foos (text) VALUES ('spawn') RETURNING *";
        let insert = SqlxEvent::<Sqlite, Foo>::query_sync(sql);
        app.world_mut().send_event(insert);

        wait_for_status(&mut app, &mut system_state, |status| {
            assert_matches!(status, SqlxEventStatus::Spawn(_, _, _));
        });
    }

    #[test]
    fn test_event_status_update() {
        let mut app = setup_app();
        let mut system_state: SystemState<
            EventReader<SqlxEventStatus<Sqlite, Foo>>,
        > = SystemState::new(app.world_mut());

        app.world_mut().spawn(Foo { id: 1, text: "update".into() });

        let sql = r#"
            INSERT INTO foos (id, text) VALUES (1, 'return')
            ON CONFLICT(id) DO UPDATE
            SET text = 'update'
            RETURNING *
        "#;
        let insert = SqlxEvent::<Sqlite, Foo>::query_sync(sql);
        app.world_mut().send_event(insert);

        wait_for_status(&mut app, &mut system_state, |status| {
            assert_matches!(status, SqlxEventStatus::Update(_, _, _));
        });
    }

    #[test]
    fn test_event_status_return() {
        let mut app = setup_app();
        let mut system_state: SystemState<
            EventReader<SqlxEventStatus<Sqlite, Foo>>,
        > = SystemState::new(app.world_mut());

        let sql = "INSERT INTO foos (text) VALUES ('return') RETURNING *";
        let insert = SqlxEvent::<Sqlite, Foo>::query(sql);
        app.world_mut().send_event(insert);

        wait_for_status(&mut app, &mut system_state, |status| {
            assert_matches!(status,
                SqlxEventStatus::Return(_, components)
                    if components[0].text == "return");
        });
    }

    #[test]
    fn test_event_status_error() {
        let mut app = setup_app();
        let mut system_state: SystemState<
            EventReader<SqlxEventStatus<Sqlite, Foo>>,
        > = SystemState::new(app.world_mut());

        let insert = SqlxEvent::<Sqlite, Foo>::query("SELECT * FROM nope");
        let id = insert.id();
        app.world_mut().send_event(insert);

        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            app.update();
            let mut reader = system_state.get(app.world());
            if reader.read().any(|status| {
                matches!(status, SqlxEventStatus::Error(i, _) if *i == id)
            }) {
                break;
            }
            assert!(Instant::now() < deadline, "timed out waiting for error");
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    // TODO: Add tests for multicurrent in-flight events (w/ IDs)
}
