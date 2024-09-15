use bevy::prelude::*;
use bevy::{app::ScheduleRunnerPlugin, utils::Duration};
use sqlx::FromRow;
use bevy_sqlx::{SqlxPlugin, SqlxEvent};

#[derive(Component, FromRow, Debug, Default, Clone)]
struct Foo {
    id: u32,
    text: String,
    flag: bool,
}

#[derive(Resource)]
struct ExitTimer(Timer);

fn main() {
    let tick_rate = Duration::from_millis(10);
    let runner = ScheduleRunnerPlugin::run_loop(tick_rate);

    App::new()
        .add_plugins(MinimalPlugins.set(runner))
        .add_plugins(SqlxPlugin::<Foo>::default())
        .insert_resource(ExitTimer(Timer::new(tick_rate * 2, TimerMode::Once)))
        .add_systems(Startup, (delete, insert.after(delete)))
        .add_systems(Update, select)
        .add_systems(Update, exit_timer)
        .run();
}

fn delete(
    mut commands: Commands,
    mut events: EventWriter<SqlxEvent<Foo>>,
) {
    SqlxEvent::<Foo>::query("DELETE FROM foos")
        .send(&mut events)
        .trigger(&mut commands);
}

fn insert(
    mut commands: Commands,
    mut events: EventWriter<SqlxEvent<Foo>>,
) {
    SqlxEvent::<Foo>::query("INSERT INTO foos(text) VALUES ('hello world')")
        .send(&mut events)
        .trigger(&mut commands);
}

fn select(
    foo_query: Query<&Foo>,
    mut commands: Commands,
    mut events: EventWriter<SqlxEvent<Foo>>,
) {
    SqlxEvent::<Foo>::query("SELECT * FROM foos")
        .send(&mut events)
        .trigger(&mut commands);
    dbg!(foo_query.iter().collect::<Vec<&Foo>>());
}

fn exit_timer(
    time: Res<Time>,
    mut timer: ResMut<ExitTimer>,
    mut exit: EventWriter<AppExit>,
) {
    timer.0.tick(time.delta());
    if timer.0.finished() {
        exit.send(AppExit::Success);
    }
}
