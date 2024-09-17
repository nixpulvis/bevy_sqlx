//! Both writer [`SqlxEvent`] and reader [`SqlxEventStatus`]
//!
//! Sending a single [`SqlxEvent`] will start by sending it's own:
//! - [`SqlxEventStatus::Start`]
//!
//! Then, depending on how the event's task in [`SqlxTasks`] is
//! processed, one of:
//! - [`SqlxEventStatus::Spawn`]
//! - [`SqlxEventStatus::Update`]
use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool};
use sqlx::{Database, Error, Executor, IntoArguments, Pool};
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::Arc;
use crate::*;

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
///     events.send(SqlxEvent::<Sqlite, Foo>::query(sql));
/// }
/// ```
#[derive(Event, Clone)]
pub struct SqlxEvent<DB: Database, C: SqlxComponent<DB::Row>> {
    label: Option<String>,
    pub(crate) func: SqlxEventFunc<DB, C>,
    _db: PhantomData<DB>,
    _c: PhantomData<C>,
}

type SqlxEventFunc<DB, C> = Arc<dyn Fn(Pool<DB>) ->
    Pin<Box<dyn Future<Output = Result<Vec<C>, Error>> + Send>> + Send + Sync>;

impl<DB: Database + Sync, C: SqlxComponent<DB::Row>> SqlxEvent<DB, C>
where
    for<'c> &'c mut DB::Connection: Executor<'c, Database = DB>,
    for<'a> <DB as sqlx::Database>::Arguments<'a>: IntoArguments<'a, DB>,
{
    /// Construct a new [`SqlxEvent`] from the given SQL string
    ///
    /// ```
    /// use sqlx::Sqlite;
    /// use bevy_sqlx::{SqlxEvent, SqlxDummy};
    ///
    /// SqlxEvent::<Sqlite, SqlxDummy>::query("SELECT * FROM dummys");
    /// ```
    pub fn query(string: &str) -> Self {
        let arc: Arc<str> = string.into();
        Self::call(Some(string), move |db| {
            let s = arc.clone();
            async move {
                sqlx::query_as(&s).fetch_all(&db).await
            }
        })
    }

    /// Construct a new [`SqlxEvent`] from the given function with access
    /// to a [`Pool<DB>`]
    ///
    /// ```
    /// use sqlx::Sqlite;
    /// use bevy_sqlx::{SqlxEvent, SqlxDummy};
    ///
    /// SqlxEvent::<Sqlite, SqlxDummy>::call(None, move |db| { async move {
    ///     sqlx::query_as("INSERT INTO dummys (text) VALUES (?) RETURNING *")
    ///         .bind("hello")
    ///         .fetch_all(&db).await
    /// }});
    /// ```
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

    /// A useful message corresponding to this event
    pub fn label(&self) -> Option<String> {
        self.label.clone().map(|s| s.to_string())
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
///             SqlxEventStatus::Start(label) => {},
///             SqlxEventStatus::Spawn(label, id, _) => {},
///             SqlxEventStatus::Update(label, id, _) => {},
///             SqlxEventStatus::Error(label, err) => {},
///         }
///     }
/// }
/// ```
#[derive(Event, Debug)]
pub enum SqlxEventStatus<DB: Database, C: SqlxComponent<DB::Row>> {
    Start(Option<String>),
    Spawn(Option<String>, C::Column, PhantomData<DB>),
    Update(Option<String>, C::Column, PhantomData<DB>),
    Error(Option<String>, Error),
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
            status.send(SqlxEventStatus::Start(event.label()));
            let db = database.pool.clone();
            let future = (event.func)(db);
            let task = task_pool.spawn(async move { future.await });
            tasks.components.push((event.label(), task));
        }
    }
}


#[cfg(test)]
mod tests {
    use std::assert_matches::assert_matches;
    use bevy::prelude::*;
    use bevy::ecs::system::SystemState;
    use bevy::tasks::{TaskPool, AsyncComputeTaskPool};
    use sqlx::{FromRow, Sqlite};
    use crate::*;

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

    fn setup_app() -> App {
        AsyncComputeTaskPool::get_or_init(|| TaskPool::new());
        let url = "sqlite:db/sqlite.db";
        let mut app = App::new();
        app.add_plugins(SqlxPlugin::<Sqlite, Foo>::from_url(url));
        app
    }

    #[test]
    fn test_event_status() {
        let mut app = setup_app();
        let mut system_state: SystemState<(
            Query<&Foo>,
            EventReader<SqlxEventStatus<Sqlite, Foo>>,
        )> = SystemState::new(app.world_mut());

        // Send an event.
        let sql = "INSERT INTO foos (text) VALUES ('tstevtsts') RETURNING *";
        let insert = SqlxEvent::<Sqlite, Foo>::query(sql);
        app.world_mut().send_event(insert);

        // No status events yet.
        let mut reader = system_state.get(app.world()).1;
        let events = reader.read();
        assert_eq!(0, events.len());

        // Update the app once.
        app.update();

        // We should have a single started event.
        let mut reader = system_state.get(app.world()).1;
        let mut events = reader.read();
        assert_eq!(1, events.len());

        assert_matches!(events.next().unwrap(),
                        SqlxEventStatus::Start(s) if
                            s.clone()
                             .expect("event called with `query`")
                             .contains("INSERT"));

        // Wait for the task's status event.
        while no_events(&mut app, &mut system_state) {
            app.update();
        }

        // We should now have a single spawned event!
        let mut reader = system_state.get(app.world()).1;
        let mut events = reader.read();
        assert_matches!(events.next().unwrap(),
                        SqlxEventStatus::Spawn(_,_,_))
    }

    fn no_events(app: &mut App, system_state: &mut SystemState<(
        Query<&Foo>,
        EventReader<SqlxEventStatus<Sqlite, Foo>>,
    )>) -> bool
    {
        let mut reader = system_state.get(app.world()).1;
        let events = reader.read();
        events.len() == 0
    }
}
