use bevy::prelude::*;
use bevy::{app::ScheduleRunnerPlugin, utils::Duration};
use sqlx::FromRow;
use bevy_sqlx::{SqlxPlugin, SqlxPrimaryKey, SqlxEvent, SqlxData};

#[derive(Component, FromRow, Clone, Debug)]
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

#[derive(Resource)]
struct ExitTimer(Timer);

fn main() {
    let tick_rate = Duration::from_millis(10);
    let runner = ScheduleRunnerPlugin::run_loop(tick_rate);

    App::new()
        .add_plugins(MinimalPlugins.set(runner))
        .add_plugins(SqlxPlugin::<Foo>::default())
        .insert_resource(ExitTimer(Timer::new(tick_rate * 10, TimerMode::Once)))
        .add_systems(Startup, (delete, insert.after(delete)))
        .add_systems(Update, (select, update))
        .add_systems(Update, exit_timer)
        .observe(|trigger: Trigger<SqlxEvent<Foo>>,
                  foo_query: Query<&Foo>| {
            dbg!(trigger.event());
            for foo in &foo_query { dbg!(&foo); }
        })
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
    SqlxEvent::<Foo>::query("INSERT INTO foos(text) VALUES ('insert') RETURNING *")
        .send(&mut events)
        .trigger(&mut commands);
}

fn select(
    mut commands: Commands,
    mut events: EventWriter<SqlxEvent<Foo>>,
) {
    SqlxEvent::<Foo>::query("SELECT * FROM foos")
        .send(&mut events)
        .trigger(&mut commands);
}

fn update(
    time: Res<Time>,
    mut commands: Commands,
    mut events: EventWriter<SqlxEvent<Foo>>,
) {
    let text = time.elapsed().as_millis().to_string();
    SqlxEvent::<Foo>::query(&format!("UPDATE foos SET text = '{}'", text))
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
