use std::marker::{PhantomData, Unpin};
use bevy::prelude::*;
use bevy::ecs::system::SystemState;
use bevy::tasks::{block_on, AsyncComputeTaskPool, Task};
use bevy::tasks::futures_lite::future;
use sqlx::{
    Error,
    Executor,
    IntoArguments,
    Database,
    Pool,
    Row,
    FromRow,
};

pub trait SqlxComponent<R: Row>:
    SqlxPrimaryKey +
    Component +
    for<'r> FromRow<'r, R> +
    Unpin
{}
impl<C, R> SqlxComponent<R> for C
where
    C: SqlxPrimaryKey +
        Component +
        for<'r> FromRow<'r, R> +
        Unpin,
    R: Row
{}

pub trait SqlxConn<'c, DB: Database>: Executor<'c, Database = DB> {}

#[derive(Resource, Debug)]
pub struct SqlxDatabase<DB: Database> {
    pub pool: Pool<DB>
}

#[derive(Resource, Debug)]
pub struct SqlxTasks<R: Row, C: SqlxComponent<R>> {
    pub queries: Vec<(String, Task<Result<Vec<C>, Error>>)>,
    _r: PhantomData<R>,
}

impl<R: Row, C: SqlxComponent<R>> Default for SqlxTasks<R, C> {
    fn default() -> Self {
        SqlxTasks {
            queries: Vec::new(),
            _r: PhantomData::<R>,
        }
    }
}

#[derive(Event, Debug)]
pub struct SqlxEvent<DB: Database, C: SqlxComponent<DB::Row>> {
    pub query: String,
    // pub query: impl Fn(impl SqlxConn) -> SqlxComponent<DB::Row>,
    _db: PhantomData<DB>,
    _c: PhantomData<C>,
}

impl<DB: Database + Sync, C: SqlxComponent<DB::Row>> SqlxEvent<DB, C> {
    pub fn query(string: &str) -> Self {
        SqlxEvent {
            query: string.to_string(),
            _db: PhantomData::<DB>,
            _c: PhantomData::<C>,
        }
    }

    pub fn send(self, events: &mut EventWriter<SqlxEvent<DB, C>>) -> Self {
        events.send(SqlxEvent {
            query: self.query.clone(),
            _db: PhantomData::<DB>,
            _c: PhantomData::<C>,
        });
        self
    }

    pub fn trigger(self, commands: &mut Commands) -> Self {
        commands.trigger(SqlxEvent {
            query: self.query.clone(),
            _db: PhantomData::<DB>,
            _c: PhantomData::<C>,
        });
        self
    }

    pub fn bind<T>(self, value: T) -> Self {
        self
    }
}

pub trait SqlxPrimaryKey {
    type Column: PartialEq;
    fn id(&self) -> Self::Column;
}

#[derive(Reflect, Component, Debug)]
pub struct SqlxData {
    pub query: String,
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
        let pool = bevy::tasks::block_on(async {
            Pool::connect(url).await.unwrap()
        });
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
        app.insert_resource(SqlxDatabase { pool: self.pool.clone() });
        app.insert_resource(SqlxTasks::<DB::Row, C>::default());
        app.add_event::<SqlxEvent<DB, C>>();
        app.register_type::<SqlxData>();
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
        for event in events.read() {
            let task_pool = AsyncComputeTaskPool::get();
            let query = event.query.clone();
            let db = database.pool.clone();
            let q = query.clone();
            let task = task_pool.spawn(async move {
                sqlx::query_as(&q).fetch_all(&db).await
            });
            tasks.queries.push((query, task));
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

        tasks.queries.retain_mut(|(sql, task)| {
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
                                commands.entity(entity)
                                        .insert(task_component)
                                        .insert(SqlxData { query: sql.clone() });
                            } else {
                                commands.spawn((
                                    task_component,
                                    SqlxData { query: sql.clone() }
                                ));
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

#[test]
fn the_one_test() {
    use std::env;
    use rand::prelude::*;
    use bevy::tasks::TaskPool;
    use sqlx::Sqlite;

    #[derive(Component, FromRow, Debug)]
    struct Foo {
        id: u32,
        text: String,
    }
    impl SqlxPrimaryKey for Foo {
        type Column = u32;
        fn id(&self) -> Self::Column { self.id }
    }

    AsyncComputeTaskPool::get_or_init(|| TaskPool::new());

    let url = env::var("DATABASE_URL").unwrap();
    let mut app = App::new();
    app.add_plugins(SqlxPlugin::<Sqlite, Foo>::url(&url));

    let delete = SqlxEvent::<Sqlite, Foo>::query("DELETE FROM foos");
    app.world_mut().send_event(delete);

    let text: String = rand::thread_rng()
        .sample_iter(rand::distributions::Alphanumeric)
        .take(10)
        .map(char::from)
        .collect();
    let insert = SqlxEvent::<Sqlite, Foo>::query("INSERT INTO foos (text) VALUES (?) RETURNING *").bind(text);
    app.world_mut().send_event(insert);

    let mut system_state: SystemState<Query<&Foo>> = SystemState::new(app.world_mut());

    let mut tries = 0;
    let mut len = system_state.get(app.world()).iter().len();
    while !(len > 0) && tries < 1000 {
        app.update();
        len = system_state.get(app.world()).iter().len();
        tries += 1;
    }

    let query = system_state.get(app.world());
    assert_eq!(1, query.single().id);
    assert_eq!("test", query.single().text);
}
