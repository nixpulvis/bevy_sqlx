use bevy::prelude::*;
use bevy::{app::ScheduleRunnerPlugin, utils::Duration};
use bevy_sqlx::{PrimaryKey, SqlxEvent, SqlxPlugin};
use sqlx::{FromRow, Sqlite};

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
        .insert_resource(ExitTimer(Timer::new(
            tick_rate * 1000,
            TimerMode::Once,
        )))
        .add_systems(Startup, (delete, insert.after(delete)))
        .add_systems(Update, (select, update))
        .add_systems(Update, exit_timer)
        .run();
}

fn delete(mut events: EventWriter<SqlxEvent<Sqlite, Foo>>) {
    events.send(SqlxEvent::<Sqlite, Foo>::query("DELETE FROM foos"));
}

fn insert(mut events: EventWriter<SqlxEvent<Sqlite, Foo>>) {
    let sql = "INSERT INTO foos(text) VALUES ('insert') RETURNING *";
    events.send(SqlxEvent::<Sqlite, Foo>::query(sql));
}

fn select(foos: Query<&Foo>, mut events: EventWriter<SqlxEvent<Sqlite, Foo>>) {
    events.send(SqlxEvent::<Sqlite, Foo>::query("SELECT * FROM foos"));

    for foo in &foos {
        dbg!(&foo);
    }
}

fn update(time: Res<Time>, mut events: EventWriter<SqlxEvent<Sqlite, Foo>>) {
    let text = time.elapsed().as_millis().to_string();
    events.send(SqlxEvent::<Sqlite, Foo>::call(move |db| {
        let text = text.clone();
        async move {
            sqlx::query_as("UPDATE foos SET text = '?'")
                .bind(text)
                .fetch_all(&db)
                .await
        }
    }));
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
