use bevy::prelude::*;
use bevy::{app::ScheduleRunnerPlugin, utils::Duration};
use bevy_sqlx::{SqlxDatabase, SqlxEvent, SqlxEventStatus, SqlxPlugin};
use sqlx::{FromRow, Sqlite};

#[derive(FromRow, Debug)]
struct Number(u32);

#[derive(Resource)]
struct ExitTimer(Timer);

fn main() {
    let tick_rate = Duration::from_millis(1);
    let runner = ScheduleRunnerPlugin::run_loop(tick_rate);

    let url = "sqlite:db/sqlite.db";
    App::new()
        .add_plugins(MinimalPlugins.set(runner))
        .insert_resource(ExitTimer(Timer::new(
            tick_rate * 1000,
            TimerMode::Once,
        )))
        .add_plugins(SqlxPlugin::<Sqlite, Number>::from_url(url))
        .add_systems(Startup, (reset, insert.after(reset)))
        .add_systems(Update, watch_status)
        .add_systems(Update, exit_timer)
        .run();
}

fn reset(db: Res<SqlxDatabase<Sqlite>>) {
    bevy::tasks::block_on(async {
        sqlx::query("DELETE FROM bars").execute(&db.pool).await.unwrap();
        sqlx::query("DELETE FROM foos").execute(&db.pool).await.unwrap();
    });
}

fn insert(mut events: EventWriter<SqlxEvent<Sqlite, Number>>) {
    let sql = "SELECT (1)";
    events.send(SqlxEvent::<Sqlite, Number>::query(sql));
}

fn watch_status(mut statuses: EventReader<SqlxEventStatus<Sqlite, Number>>) {
    for status in statuses.read() {
        dbg!({
            "status";
            status
        });
    }
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
