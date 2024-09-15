# Bevy SQLx
-----
![Bevy SQLx Demo](./bevy_sqlx.gif)

Bevy SQLx is a database plugin for Bevy's ECS which allows for SQL queries to
be performed and data entities to be spawned.


### Example

```rust
use std::env;
use bevy::prelude::*;
use sqlx::{FromRow, Sqlite, SqlitePool};
use bevy_sqlx::{SqlxPlugin, SqlxPrimaryKey, SqlxEvent};

#[derive(Component, FromRow, Debug)]
struct MyTable {
    id: u32,
    flag: bool,
    text: String,
}

impl SqlxPrimaryKey for MyTable {
    type Column = u32;

    fn id(&self) -> Self::Column {
        self.id
    }
}

fn main() {
    let pool = bevy::tasks::block_on(async {
        let url = env::var("DATABASE_URL").unwrap();
        SqlitePool::connect(&url).await.unwrap()
    });

    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(SqlxPlugin::<Sqlite, MyTable>::default())
        .add_systems(Startup, insert)
        .add_systems(Update, query)
        .run();
}

fn insert(
    mut commands: Commands,
    mut events: EventWriter<SqlxEvent<Sqlite, MyTable>>,
) {
    let sql = "INSERT INTO mytable(text) VALUES ('insert') RETURNING *";
    SqlxEvent::<Sqlite, MyTable>::query(sql)
        .send(&mut events)
        .trigger(&mut commands);
}

fn query(mut my_tables: Query<&MyTable>) {
    for my_table in &my_tables {
        dbg(!my_table)
    }
}
```

### Usage

```sh
DATABASE_URL="sqlite:db/sqlite.db" cargo sqlx database setup
DATABASE_URL="sqlite:db/sqlite.db" cargo test
DATABASE_URL="sqlite:db/sqlite.db" \
cargo run --example minimal --features bevy/bevy_winit,bevy/wayland
```
