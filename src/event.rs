use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool};
use sqlx::{Database, Error, Executor, IntoArguments, Pool};
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::Arc;
use crate::*;

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

type SqlxEventFunc<DB, C> = Arc<dyn Fn(Pool<DB>) ->
    Pin<Box<dyn Future<Output = Result<Vec<C>, Error>> + Send>> + Send + Sync>;

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

    pub fn label(&self) -> Option<String> {
        self.label.clone().map(|s| s.to_string())
    }

    pub(crate) fn func(&self) -> &SqlxEventFunc<DB, C> {
        &self.func
    }
}

#[derive(Event, Debug, PartialEq)]
pub enum SqlxEventStatus<DB: Database, C: SqlxComponent<DB::Row>> {
    Started(Option<String>),
    Spawn(C::Column, PhantomData<DB>),
    Insert(C::Column, PhantomData<DB>),
    Error,
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
        mut status: EventWriter<SqlxEventStatus<DB, C>>,
    ) {
        let task_pool = AsyncComputeTaskPool::get();
        for event in events.read() {
            status.send(SqlxEventStatus::Started(event.label()));
            let db = database.pool.clone();
            let future = (event.func())(db);
            let task = task_pool.spawn(async move { future.await });
            tasks.components.push(task);
        }
    }
}


#[cfg(test)]
mod tests {
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
        app.add_plugins(SqlxPlugin::<Sqlite, Foo>::url(url));
        app
    }

    #[test]
    fn test_event_status() {
        let mut app = setup_app();
        let mut system_state: SystemState<(
            Query<&Foo>,
            EventReader<SqlxEventStatus<Sqlite, Foo>>,
        )> = SystemState::new(app.world_mut());

        // Sent an event.
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
        match events.next().unwrap() {
            SqlxEventStatus::Started(s) => {
                assert!(s.clone().expect("event called with `query`")
                         .contains("INSERT"));
            },
            _ => { panic!("bad status event"); }
        }


        // Wait for the task's status event.
        for _ in 0..100 { dbg!(app.update()) }

        let mut system_state: SystemState<(
            Query<&Foo>,
            EventReader<SqlxEventStatus<Sqlite, Foo>>,
        )> = SystemState::new(app.world_mut());
        let mut reader = system_state.get(app.world()).1;
        let mut events = reader.read();
        assert_eq!(1, events.len());
        match events.next().unwrap() {
            SqlxEventStatus::Spawn(id,_) => {
                assert_eq!(1, *id);
            },
            _ => { panic!("bad status event"); }
        }
    }
}
