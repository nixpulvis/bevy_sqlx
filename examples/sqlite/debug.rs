use std::sync::Arc;
use rand::prelude::*;
use bevy::prelude::*;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use sqlx::{FromRow, Sqlite};
use bevy_sqlx::{
    SqlxPlugin,
    SqlxEvent,
    SqlxEventStatus,
    PrimaryKey,
};

#[derive(Reflect, Component, FromRow, Debug, Default, Clone)]
#[allow(unused)]
struct Foo {
    id: u32,
    text: String,
    flag: bool,
}

impl PrimaryKey for Foo {
    type Column = u32;

    fn primary_key(&self) -> Self::Column {
        self.id
    }
}

pub struct FooPlugin;

impl Plugin for FooPlugin {
    fn build(&self, app: &mut App) {
        let url = "sqlite:db/sqlite.db";
        app.add_plugins(SqlxPlugin::<Sqlite, Foo>::url(url));
        app.add_systems(Update, Self::send_foo_events);
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
            for (entity, _foo) in foos_query.iter() {
                commands.entity(entity).despawn_recursive();
            }
        }

        if keys.just_pressed(KeyCode::KeyF) && keys.just_pressed(KeyCode::KeyI) {
            SqlxEvent::<Sqlite, Foo>::call(Some("INSERT"), move |db| { async move {
                let text: String = rand::thread_rng()
                    .sample_iter(rand::distributions::Alphanumeric)
                    .take(10)
                    .map(char::from)
                    .collect();
                sqlx::query_as("INSERT INTO foos (text) VALUES (?) RETURNING *")
                    .bind(text)
                    .fetch_all(&db).await
            }}).send(&mut events).trigger(&mut commands);
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

impl PrimaryKey for Bar {
    type Column = u32;

    fn primary_key(&self) -> Self::Column {
        self.id
    }
}


pub struct BarPlugin;

impl Plugin for BarPlugin {
    fn build(&self, app: &mut App) {
        let url = "sqlite:db/sqlite.db";
        app.add_plugins(SqlxPlugin::<Sqlite, Bar>::url(&url));
        app.add_systems(Update, Self::send_bar_events);
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
            for (entity, _bar) in bars_query.iter() {
                commands.entity(entity).despawn_recursive();
            }
        }

        if keys.just_pressed(KeyCode::KeyB) && keys.just_pressed(KeyCode::KeyI) {
            // Choose a random Foo to be associated with
            if let Some(foo) = foos_query.iter().choose(&mut rand::thread_rng()) {
                let foo: Arc<Foo> = foo.clone().into();
                let sql = "INSERT INTO bars (foo_id) VALUES (?) RETURNING *";
                SqlxEvent::<Sqlite, Bar>::call(Some(sql), move |db| {
                    let foo = foo.clone();
                    async move {
                        sqlx::query_as(sql)
                            .bind(foo.id)
                            .fetch_all(&db).await
                    }
                }).send(&mut events).trigger(&mut commands);
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
        .add_systems(Update, (watch_status,
                              detect_changed,
                              detect_removals))
        .run();
}

fn watch_status(mut statuses: EventReader<SqlxEventStatus<Sqlite, Foo>>) {
    for status in statuses.read() {
        dbg!({ "status"; status });
    }
}

fn detect_changed(
    foo_query: Query<(Entity, &Foo), Changed<Foo>>,
    bar_query: Query<(Entity, &Bar), Changed<Bar>>,
) {
    for foo in &foo_query {
        dbg!({ "foo changed"; &foo});
    }
    for bar in &bar_query {
        dbg!({ "bar changed"; &bar});
    }
}

fn detect_removals(
    mut foo_removals: RemovedComponents<Foo>,
    mut bar_removals: RemovedComponents<Bar>,
) {
    for entity in foo_removals.read() {
        dbg!({ "foo removed"; entity });
    }
    for entity in bar_removals.read() {
        dbg!({ "bar removed"; entity });
    }
}
