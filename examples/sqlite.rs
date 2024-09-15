use std::time::Duration;
use bevy::prelude::*;
use sqlx::FromRow;
use bevy_sqlx::{SqlxPlugin, SqlxFetchEvent};

#[derive(Component, FromRow, Debug, Default)]
struct Foo {
    id: u32,
    text: String,
    flag: bool,
}

#[derive(Component, FromRow, Debug, Default)]
struct Bar {

}

#[derive(Component, Debug)]
struct FetchTimer(Timer);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(SqlxPlugin::<Foo>::default())
        .add_plugins(SqlxPlugin::<Bar>::default())
        .add_systems(Startup, spawn_fetch_timer)
        .add_systems(Update, send_fetch)
        .add_systems(Update, query_spawned)
        .run();
}

fn spawn_fetch_timer(
    time: Res<Time>,
    mut commands: Commands,
) {
    let timer = Timer::new(Duration::from_secs(1), TimerMode::Repeating);
    dbg!("spawning 1 second timer");
    commands.spawn(FetchTimer(timer));
}

fn send_fetch(
    time: Res<Time>,
    mut timer_query: Query<(Entity, &mut FetchTimer)>,
    mut commands: Commands,
    mut events: EventWriter<SqlxFetchEvent>,
) {
    for (timer_entity, mut timer) in timer_query.iter_mut() {
        timer.0.tick(time.delta());

        if timer.0.finished() {
            events.send(SqlxFetchEvent::query("DELETE FROM foos"));
            events.send(SqlxFetchEvent::query("SELECT id, text, flag FROM foos"));
            events.send(SqlxFetchEvent::query(r#"
                    INSERT INTO foos(id, text, flag)
                    VALUES (8, 'hello world', 0)"#));
            events.send(SqlxFetchEvent::query("SELECT id, text, flag FROM foos"));
            events.send(SqlxFetchEvent::query("DELETE FROM foos"));

            dbg!("timer fired");
            commands.entity(timer_entity).despawn();
        }
    }
}

fn query_spawned(
    mut foo_query: Query<(Entity, &Foo)>,
    mut bar_query: Query<(Entity, &Bar)>,
) {
    dbg!("query");
    for (entity, foo) in &mut foo_query {
        dbg!(entity, foo);
    }

    for (entity, bar) in &mut bar_query {
        dbg!(entity, bar);
    }
}
