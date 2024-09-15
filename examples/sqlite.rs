use std::env;
use rand::prelude::*;
use bevy::prelude::*;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use sqlx::FromRow;
use sqlx::{Sqlite, SqlitePool};
use bevy_sqlx::{SqlxPlugin, SqlxPrimaryKey, SqlxEvent};

#[derive(Reflect, Component, FromRow, Debug, Default, Clone)]
struct Foo {
    id: u32,
    text: String,
    flag: bool,
}

impl SqlxPrimaryKey for Foo {
    type Column = u32;

    fn id(&self) -> Self::Column {
        self.id
    }
}

pub struct FooPlugin;

impl Plugin for FooPlugin {
    fn build(&self, app: &mut App) {
        let pool = bevy::tasks::block_on(async {
            let url = env::var("DATABASE_URL").unwrap();
            SqlitePool::connect(&url).await.unwrap()
        });

        app.add_plugins(SqlxPlugin::<Sqlite, Foo>::new(pool));
        app.add_systems(Update, Self::send_foo_events);
        app.observe(|trigger: Trigger<SqlxEvent<Sqlite, Foo>>,
                  foo_query: Query<&Foo>| {
            dbg!({ "observe"; (trigger.event(), &foo_query.iter().len()) });
            for foo in &mut foo_query.iter() {
                dbg!({ "observe"; &foo });
            }
        });
    }
}

impl FooPlugin {
    fn send_foo_events(
        foos_query: Query<(Entity, &Foo)>,
        keys: Res<ButtonInput<KeyCode>>,
        mut commands: Commands,
        mut events: EventWriter<SqlxEvent<Sqlite, Foo>>,
    ) {
        if keys.just_pressed(KeyCode::KeyF) && keys.just_pressed(KeyCode::KeyD) {
            SqlxEvent::<Sqlite, Foo>::query("DELETE FROM foos")
                .send(&mut events)
                .trigger(&mut commands);
            for (entity, foo) in foos_query.iter() {
                dbg!(&foo);
                commands.entity(entity).despawn_recursive();
            }
        }

        if keys.just_pressed(KeyCode::KeyF) && keys.just_pressed(KeyCode::KeyI) {
            let text: String = rand::thread_rng()
                .sample_iter(rand::distributions::Alphanumeric)
                .take(10)
                .map(char::from)
                .collect();

            SqlxEvent::<Sqlite, Foo>::query(
                    &format!("INSERT INTO foos(text) VALUES ('{}') RETURNING *", text))
                .send(&mut events)
                .trigger(&mut commands);
            // TODO: Should use bind.
            // let sql = r#"
            //     INSERT INTO foos(id, text, flag)
            //     VALUES (8, '?', 0)
            // "#;
            // events.send(SqlxEvent::<Sqlite, Foo>::query(&sql).bind(text));
        }

        if keys.just_pressed(KeyCode::KeyF) && keys.just_pressed(KeyCode::KeyS) {
            SqlxEvent::<Sqlite, Foo>::query("SELECT id, text, flag FROM foos")
                .send(&mut events)
                .trigger(&mut commands);
        }
    }
}


#[derive(Reflect, Component, FromRow, Debug, Default, Clone)]
struct Bar {
    id: u32,
    foo_id: u32,
    optional: Option<String>,
}

impl SqlxPrimaryKey for Bar {
    type Column = u32;

    fn id(&self) -> Self::Column {
        self.id
    }
}


pub struct BarPlugin;

impl Plugin for BarPlugin {
    fn build(&self, app: &mut App) {
        let pool = bevy::tasks::block_on(async {
            let url = env::var("DATABASE_URL").unwrap();
            SqlitePool::connect(&url).await.unwrap()
        });

        app.add_plugins(SqlxPlugin::<Sqlite, Bar>::new(pool));
        app.add_systems(Update, Self::send_bar_events);
        app.observe(|trigger: Trigger<SqlxEvent<Sqlite, Bar>>,
                  bar_query: Query<&Bar>| {
            dbg!(trigger.event(), bar_query);
        });
    }
}

impl BarPlugin {
    fn send_bar_events(
        bars_query: Query<(Entity, &Bar)>,
        foos_query: Query<&Foo>,
        keys: Res<ButtonInput<KeyCode>>,
        mut commands: Commands,
        mut events: EventWriter<SqlxEvent<Sqlite, Bar>>,
    ) {
        if keys.just_pressed(KeyCode::KeyB) && keys.just_pressed(KeyCode::KeyD) {
            SqlxEvent::<Sqlite, Bar>::query("DELETE FROM bars")
                .send(&mut events)
                .trigger(&mut commands);
            for (entity, bar) in bars_query.iter() {
                dbg!(&bar);
                commands.entity(entity).despawn_recursive();
            }
        }

        if keys.just_pressed(KeyCode::KeyB) && keys.just_pressed(KeyCode::KeyI) {
            // Choose a random Foo to be associated with.
            if let Some(foo) = foos_query.iter().choose(&mut rand::thread_rng()) {
                SqlxEvent::<Sqlite, Bar>::query(
                    &format!("INSERT INTO bars(foo_id) VALUES ({}) RETURNING *", foo.id))
                    .send(&mut events)
                    .trigger(&mut commands);
            } else {
                dbg!("No Foo to choose from.");
            }
        }

        if keys.just_pressed(KeyCode::KeyB) && keys.just_pressed(KeyCode::KeyS) {
            SqlxEvent::<Sqlite, Bar>::query("SELECT * FROM bars")
                .send(&mut events)
                .trigger(&mut commands);
        }
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(WorldInspectorPlugin::new())
        .add_plugins(FooPlugin)
        .add_plugins(BarPlugin)
        .register_type::<Foo>()
        .register_type::<Bar>()
        .add_systems(Update, query_spawned)
        .run();
}

fn query_spawned(
    keys: Res<ButtonInput<KeyCode>>,
    mut foo_query: Query<(Entity, &Foo)>,
    mut bar_query: Query<(Entity, &Bar)>,
) {
    if keys.just_pressed(KeyCode::KeyF) &&
       keys.just_pressed(KeyCode::KeyQ)
    {
        dbg!(&foo_query);
        for foo in &mut foo_query { dbg!(&foo); }
    }

    if keys.just_pressed(KeyCode::KeyB) &&
       keys.just_pressed(KeyCode::KeyQ)
    {
        dbg!(&bar_query);
        for bar in &mut bar_query { dbg!(&bar); }
    }
}
