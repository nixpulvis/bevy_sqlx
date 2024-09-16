use bevy::ecs::system::SystemState;
use bevy::prelude::*;
use bevy::tasks::futures_lite::future;
use bevy::tasks::{block_on, AsyncComputeTaskPool, Task};
use sqlx::{Database, Error, Executor, FromRow, IntoArguments, Pool, Row};
use std::future::Future;
use std::marker::{PhantomData, Unpin};
use std::pin::Pin;
use std::sync::Arc;

pub trait SqlxComponent<R: Row>:
    SqlxPrimaryKey + Component + for<'r> FromRow<'r, R> + Unpin
{
}
impl<C, R> SqlxComponent<R> for C
where
    C: SqlxPrimaryKey + Component + for<'r> FromRow<'r, R> + Unpin,
    R: Row,
{
}

#[derive(Resource, Debug)]
pub struct SqlxDatabase<DB: Database> {
    pub pool: Pool<DB>,
}

#[derive(Resource, Debug)]
pub struct SqlxTasks<R: Row, C: SqlxComponent<R>> {
    pub components: Vec<Task<Result<Vec<C>, Error>>>,
    _r: PhantomData<R>,
}

impl<R: Row, C: SqlxComponent<R>> Default for SqlxTasks<R, C> {
    fn default() -> Self {
        SqlxTasks {
            components: Vec::new(),
            _r: PhantomData::<R>,
        }
    }
}

#[derive(Event, Clone)]
pub struct SqlxEvent<DB: Database, C: SqlxComponent<DB::Row>> {
    label: Option<String>,
    call: Arc<dyn Fn(Pool<DB>) -> Pin<Box<dyn Future<Output = Result<Vec<C>, Error>> + Send>> + Send + Sync>,
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
            call: Arc::new(move |db: Pool<DB>| {
                Box::pin(func(db))
            }),
            _db: PhantomData::<DB>,
            _c: PhantomData::<C>,
        }
    }

    pub fn send(self, events: &mut EventWriter<SqlxEvent<DB, C>>) -> Self {
        events.send(SqlxEvent {
            label: self.label.clone(),
            call: self.call.clone(),
            _db: PhantomData::<DB>,
            _c: PhantomData::<C>,
        });
        self
    }

    pub fn trigger(self, commands: &mut Commands) -> Self {
        commands.trigger(SqlxEvent {
            label: self.label.clone(),
            call: self.call.clone(),
            _db: PhantomData::<DB>,
            _c: PhantomData::<C>,
        });
        self
    }

    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }
}

pub trait SqlxPrimaryKey {
    type Column: PartialEq;
    fn id(&self) -> Self::Column;
}

pub struct SqlxPlugin<DB: Database, C: SqlxComponent<DB::Row>> {
    pool: Pool<DB>,
    _c: PhantomData<C>,
}

impl<DB: Database, C: SqlxComponent<DB::Row>> SqlxPlugin<DB, C> {
    pub fn pool(pool: Pool<DB>) -> Self {
        SqlxPlugin {
            pool,
            _c: PhantomData,
        }
    }

    pub fn url(url: &str) -> Self {
        let pool = bevy::tasks::block_on(async { Pool::connect(url).await.unwrap() });
        SqlxPlugin {
            pool,
            _c: PhantomData,
        }
    }
}

impl<DB: Database + Sync, C: SqlxComponent<DB::Row>> Plugin for SqlxPlugin<DB, C>
where
    for<'c> &'c mut <DB as Database>::Connection: Executor<'c, Database = DB>,
    for<'q> <DB as Database>::Arguments<'q>: IntoArguments<'q, DB>,
{
    fn build(&self, app: &mut App) {
        app.insert_resource(SqlxDatabase {
            pool: self.pool.clone(),
        });
        app.insert_resource(SqlxTasks::<DB::Row, C>::default());
        app.add_event::<SqlxEvent<DB, C>>();
        app.add_systems(Update, (Self::tasks, Self::entities));
    }
}

impl<DB: Database + Sync, C: SqlxComponent<DB::Row>> SqlxPlugin<DB, C>
where
    for<'c> &'c mut <DB as Database>::Connection: Executor<'c, Database = DB>,
    for<'q> <DB as Database>::Arguments<'q>: IntoArguments<'q, DB>,
{
    pub fn tasks(
        database: Res<SqlxDatabase<DB>>,
        mut tasks: ResMut<SqlxTasks<DB::Row, C>>,
        mut events: EventReader<SqlxEvent<DB, C>>,
    ) {
        let task_pool = AsyncComputeTaskPool::get();
        for event in events.read() {
            let db = database.pool.clone();
            let future = (event.call)(db);
            let task = task_pool.spawn(async move { future.await });
            tasks.components.push(task);
        }
    }

    pub fn entities(
        world: &mut World,
        params: &mut SystemState<(
            Query<(Entity, Ref<C>)>,
            Commands,
            ResMut<SqlxTasks<DB::Row, C>>,
        )>,
    ) {
        let (mut query, mut commands, mut tasks) = params.get_mut(world);

        // for (entity, component) in &mut query {
        //     // TODO: Send Encoded UPDATE or callback function?
        //     // TODO: Need a dirty bit to check so we don't send just
        //     //       received updated entities.
        //     if component.is_changed() && !component.is_added() {
        //         dbg!("TODO: UPDATE");
        //     }
        // }

        tasks.components.retain_mut(|task| {
            let status = block_on(future::poll_once(task));
            let retain = status.is_none();
            if let Some(result) = status {
                match result {
                    Ok(task_components) => {
                        // TODO: Look into world.spawn_batch after taking set disjunction of ids.
                        for task_component in task_components {
                            // Check if the task's component is already spawned.
                            let mut existing_entity = None;
                            for (entity, spawned_component) in &mut query {
                                if task_component.id() == spawned_component.id() {
                                    existing_entity = Some(entity);
                                    break;
                                }
                            }

                            if let Some(entity) = existing_entity {
                                commands.entity(entity).insert(task_component);
                            } else {
                                commands.spawn(task_component);
                            }
                        }
                    }
                    Err(err) => {
                        dbg!(err);
                    }
                }
            }
            retain
        });

        params.apply(world);
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::*;
    use bevy::tasks::TaskPool;
    use sqlx::{FromRow, Sqlite};
    use crate::*;

    #[derive(Component, FromRow, Debug)]
    struct Foo {
        id: u32,
        text: String,
    }

    impl SqlxPrimaryKey for Foo {
        type Column = u32;
        fn id(&self) -> Self::Column {
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
    fn test_query() {
        let mut app = setup_app();
        let mut system_state: SystemState<Query<&Foo>> = SystemState::new(app.world_mut());

        let sql = "INSERT INTO foos (text) VALUES ('test query') RETURNING *";
        let insert = SqlxEvent::<Sqlite, Foo>::query(sql);
        app.world_mut().send_event(insert);

        let mut tries = 0;
        let mut len = system_state.get(app.world()).iter().len();
        while !(len > 0) && tries < 1000 {
            app.update();
            len = system_state.get(app.world()).iter().len();
            tries += 1;
        }

        let query = system_state.get(app.world());
        assert_eq!("test query", query.single().text);
    }

    #[test]
    fn test_callback() {
        let mut app = setup_app();
        let mut system_state: SystemState<Query<&Foo>> = SystemState::new(app.world_mut());

        let delete = SqlxEvent::<Sqlite, Foo>::query("DELETE FROM foos");
        app.world_mut().send_event(delete);

        let text = "test callback";
        let insert = SqlxEvent::<Sqlite, Foo>::call(None, move |db| { async move {
            sqlx::query_as("INSERT INTO foos (text) VALUES (?) RETURNING *")
                .bind(text)
                .fetch_all(&db).await
        }});
        app.world_mut().send_event(insert);

        let mut tries = 0;
        let mut len = system_state.get(app.world()).iter().len();
        while !(len > 0) && tries < 1000 {
            app.update();
            len = system_state.get(app.world()).iter().len();
            tries += 1;
        }

        let query = system_state.get(app.world());
        assert_eq!(text, query.single().text);
    }
}
