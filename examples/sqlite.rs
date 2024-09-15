use rand::prelude::*;
use bevy::prelude::*;
use sqlx::FromRow;
use bevy_sqlx::{SqlxPlugin, SqlxEvent};

#[derive(Component, FromRow, Debug, Default, Clone)]
struct Foo {
    id: u32,
    text: String,
    flag: bool,
}

#[derive(Component, FromRow, Debug, Default, Clone)]
struct Bar {
    foo_id: u32,
    optional: Option<String>,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(SqlxPlugin::<Foo>::default())
        .add_plugins(SqlxPlugin::<Bar>::default())
        .add_systems(Update, send_foo_events)
        .add_systems(Update, send_bar_events)
        .add_systems(Update, query_spawned)
        .observe(|trigger: Trigger<SqlxEvent<Bar>>,
                  bar_query: Query<&Bar>| {
            dbg!(trigger.event(), bar_query);
        })
        .observe(|trigger: Trigger<SqlxEvent<Foo>>,
                  foo_query: Query<&Foo>| {
            dbg!(trigger.event(), foo_query);
        })
        .run();
}

fn send_bar_events(
    foos_query: Query<&Foo>,
    keys: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    mut events: EventWriter<SqlxEvent<Bar>>,
) {
    if keys.pressed(KeyCode::KeyB) && keys.pressed(KeyCode::KeyD) {
        SqlxEvent::<Bar>::query("DELETE FROM bars")
            .send(&mut events)
            .trigger(&mut commands);
    }

    if keys.pressed(KeyCode::KeyB) && keys.pressed(KeyCode::KeyI) {
        if let Some(foo) = foos_query.iter().choose(&mut rand::thread_rng()) {
            SqlxEvent::<Bar>::query(
                &format!("INSERT INTO bars(foo_id) VALUES ({})", foo.id))
                .send(&mut events)
                .trigger(&mut commands);
        } else {
            dbg!("No Foo to choose from.");
        }
    }

    if keys.pressed(KeyCode::KeyB) && keys.pressed(KeyCode::KeyS) {
        SqlxEvent::<Bar>::query("SELECT * FROM bars")
            .send(&mut events)
            .trigger(&mut commands);
    }
}

fn send_foo_events(
    keys: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    mut events: EventWriter<SqlxEvent<Foo>>,
) {
    if keys.pressed(KeyCode::KeyF) && keys.pressed(KeyCode::KeyD) {
        SqlxEvent::<Foo>::query("DELETE FROM foos")
            .send(&mut events)
            .trigger(&mut commands);
    }

    if keys.pressed(KeyCode::KeyF) && keys.pressed(KeyCode::KeyI) {
        let text: String = rand::thread_rng()
            .sample_iter(rand::distributions::Alphanumeric)
            .take(10)
            .map(char::from)
            .collect();

        SqlxEvent::<Foo>::query(
                &format!("INSERT INTO foos(text) VALUES ('{}')", text))
            .send(&mut events)
            .trigger(&mut commands);
        // TODO: Should use bind.
        // let sql = r#"
        //     INSERT INTO foos(id, text, flag)
        //     VALUES (8, '?', 0)
        // "#;
        // events.send(SqlxEvent::<Foo>::query(&sql).bind(text));
    }

    if keys.pressed(KeyCode::KeyF) && keys.pressed(KeyCode::KeyS) {
        SqlxEvent::<Foo>::query("SELECT id, text, flag FROM foos")
            .send(&mut events)
            .trigger(&mut commands);
    }
}

// TODO: Find a better way to show the loaded data.
fn query_spawned(
    keys: Res<ButtonInput<KeyCode>>,
    mut foo_query: Query<(Entity, &Foo)>,
    mut bar_query: Query<(Entity, &Bar)>,
) {
    if keys.pressed(KeyCode::KeyF) && keys.pressed(KeyCode::KeyQ) {
        for (_entity, foo) in &mut foo_query {
            dbg!(foo);
        }
    }

    if keys.pressed(KeyCode::KeyB) && keys.pressed(KeyCode::KeyQ) {
        for (_entity, bar) in &mut bar_query {
            dbg!(bar);
        }
    }
}
