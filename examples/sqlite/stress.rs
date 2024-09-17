use bevy::prelude::*;
use bevy::{app::ScheduleRunnerPlugin, utils::Duration};
use sqlx::{FromRow, Sqlite};
use bevy_sqlx::{SqlxPlugin, PrimaryKey, SqlxEvent};

#[allow(unused_variables, dead_code)]
#[derive(Component, FromRow, Debug)]
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

#[derive(Resource)]
struct ExitTimer(Timer);

fn main() {
    let tick_rate = Duration::from_millis(1);
    let runner = ScheduleRunnerPlugin::run_loop(tick_rate);

    let url = "sqlite:db/sqlite.db";
    App::new()
        .add_plugins(MinimalPlugins.set(runner))
        .add_plugins(SqlxPlugin::<Sqlite, Foo>::from_url(url))
        .insert_resource(ExitTimer(Timer::new(tick_rate * 1000, TimerMode::Once)))
        .add_systems(Startup, (delete, insert.after(delete)))
        .add_systems(Update, (select, update))
        .add_systems(Update, exit_timer)
        .run();
}

fn delete(
    mut commands: Commands,
    mut events: EventWriter<SqlxEvent<Sqlite, Foo>>,
) {
    SqlxEvent::<Sqlite, Foo>::query("DELETE FROM foos")
        .send(&mut events)
        .trigger(&mut commands);
}

fn insert(
    mut commands: Commands,
    mut events: EventWriter<SqlxEvent<Sqlite, Foo>>,
) {
    SqlxEvent::<Sqlite, Foo>::query("INSERT INTO foos(text) VALUES ('insert') RETURNING *")
        .send(&mut events)
        .trigger(&mut commands);
}

fn select(
    foos: Query<&Foo>,
    mut commands: Commands,
    mut events: EventWriter<SqlxEvent<Sqlite, Foo>>,
) {
    SqlxEvent::<Sqlite, Foo>::query("SELECT * FROM foos")
        .send(&mut events)
        .trigger(&mut commands);
    for foo in &foos {
        dbg!(&foo);
    }
}

fn update(
    time: Res<Time>,
    mut commands: Commands,
    mut events: EventWriter<SqlxEvent<Sqlite, Foo>>,
) {
    let text = time.elapsed().as_millis().to_string();
    SqlxEvent::<Sqlite, Foo>::query(&format!("UPDATE foos SET text = '{}'", text))
        .send(&mut events)
        .trigger(&mut commands);
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
