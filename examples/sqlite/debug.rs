use bevy::prelude::*;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_sqlx::{PrimaryKey, SqlxEvent, SqlxEventStatus, SqlxPlugin};
use rand::prelude::*;
use sqlx::{FromRow, Sqlite};
use std::sync::Arc;

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
        app.add_plugins(SqlxPlugin::<Sqlite, Foo>::from_url(url));
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
        if keys.pressed(KeyCode::KeyF) && keys.just_pressed(KeyCode::KeyD) {
            events
                .send(SqlxEvent::<Sqlite, Foo>::query_sync("DELETE FROM foos"));
            for (entity, _foo) in foos_query.iter() {
                commands.entity(entity).despawn_recursive();
            }
        }

        if keys.pressed(KeyCode::KeyF) && keys.just_pressed(KeyCode::KeyI) {
            events.send(SqlxEvent::<Sqlite, Foo>::call_sync(
                move |db| async move {
                    let text: String = rand::thread_rng()
                        .sample_iter(rand::distributions::Alphanumeric)
                        .take(10)
                        .map(char::from)
                        .collect();
                    sqlx::query_as(
                        "INSERT INTO foos (text) VALUES (?) RETURNING *",
                    )
                    .bind(text)
                    .fetch_all(&db)
                    .await
                },
            ));
        }

        if keys.pressed(KeyCode::KeyF) && keys.just_pressed(KeyCode::KeyS) {
            events.send(SqlxEvent::<Sqlite, Foo>::query_sync(
                "SELECT id, text, flag FROM foos",
            ));
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
        app.add_plugins(SqlxPlugin::<Sqlite, Bar>::from_url(&url));
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
        if keys.pressed(KeyCode::KeyB) && keys.just_pressed(KeyCode::KeyD) {
            events
                .send(SqlxEvent::<Sqlite, Bar>::query_sync("DELETE FROM bars"));
            for (entity, _bar) in bars_query.iter() {
                commands.entity(entity).despawn_recursive();
            }
        }

        if keys.pressed(KeyCode::KeyB) && keys.just_pressed(KeyCode::KeyI) {
            // Choose a random Foo to be associated with
            if let Some(foo) = foos_query.iter().choose(&mut rand::thread_rng())
            {
                let foo: Arc<Foo> = foo.clone().into();
                let sql = "INSERT INTO bars (foo_id) VALUES (?) RETURNING *";
                events.send(SqlxEvent::<Sqlite, Bar>::call_sync(move |db| {
                    let foo = foo.clone();
                    async move {
                        sqlx::query_as(sql).bind(foo.id).fetch_all(&db).await
                    }
                }));
            } else {
                dbg!("No Foo to choose from.");
            }
        }

        if keys.pressed(KeyCode::KeyB) && keys.just_pressed(KeyCode::KeyS) {
            events.send(SqlxEvent::<Sqlite, Bar>::query_sync(
                "SELECT * FROM bars",
            ));
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
        .add_systems(Update, (watch_status, detect_changed, detect_removals))
        .run();
}

fn watch_status(
    mut foo_statuses: EventReader<SqlxEventStatus<Sqlite, Foo>>,
    mut bar_statuses: EventReader<SqlxEventStatus<Sqlite, Bar>>,
) {
    for foo_status in foo_statuses.read() {
        dbg!({
            "Foo status";
            foo_status
        });
    }
    for bar_status in bar_statuses.read() {
        dbg!({
            "Bar status";
            bar_status
        });
    }
}

fn detect_changed(
    foo_query: Query<(Entity, &Foo), Changed<Foo>>,
    bar_query: Query<(Entity, &Bar), Changed<Bar>>,
) {
    for foo in &foo_query {
        dbg!({
            "Foo changed";
            &foo
        });
    }
    for bar in &bar_query {
        dbg!({
            "Bar changed";
            &bar
        });
    }
}

fn detect_removals(
    mut foo_removals: RemovedComponents<Foo>,
    mut bar_removals: RemovedComponents<Bar>,
) {
    for entity in foo_removals.read() {
        dbg!({
            "foo removed";
            entity
        });
    }
    for entity in bar_removals.read() {
        dbg!({
            "bar removed";
            entity
        });
    }
}
