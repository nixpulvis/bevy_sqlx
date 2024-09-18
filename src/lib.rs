#![feature(assert_matches)]
#![allow(unexpected_cfgs)]
//! Bevy SQLx is a database plugin for Bevy's ECS which allows for SQL queries
//! to be performed and data entities to be spawned and managed.
//!
//! ### Setup
//!
//! - Define a [`Component`](bevy::prelude::Component) with
//! [`FromRow`](sqlx::FromRow) and [`PrimaryKey`]
//!
//! ```
//! # use bevy::prelude::*;
//! # use sqlx::{FromRow, Sqlite};
//! # use bevy_sqlx::{SqlxPlugin, SqlxEvent, PrimaryKey};
//! #
//! #[derive(Component, FromRow)]
//! struct Foo {
//!     id: u32,
//!     flag: bool,
//!     text: String,
//! }
//!
//! impl PrimaryKey for Foo {
//!     type Column = u32;
//!     fn primary_key(&self) -> Self::Column { self.id }
//! }
//! ```
//!
//! - Add the [`SqlxPlugin`] to the [`App`](bevy::prelude::App)
//!
//! ```
//! # use bevy::prelude::*;
//! # use sqlx::{FromRow, Sqlite};
//! # use bevy_sqlx::{SqlxPlugin, SqlxEvent, PrimaryKey};
//! # #[derive(Component, FromRow)]
//! # struct Foo(u32);
//! # impl PrimaryKey for Foo {
//! #     type Column = u32;
//! #     fn primary_key(&self) -> Self::Column { self.0 }
//! # }
//! let url = "sqlite:db/sqlite.db";
//! let app = App::new()
//!     .add_plugins(DefaultPlugins)
//!     .add_plugins(SqlxPlugin::<Sqlite, Foo>::from_url(&url))
//!     .run();
//! ```
//!
//! ### Usage
//!
//! - Send [`SqlxEvent`] events to query the database
//!
//! ```
//! # use bevy::prelude::*;
//! # use sqlx::{FromRow, Sqlite};
//! # use bevy_sqlx::{SqlxPlugin, SqlxEvent, PrimaryKey};
//! # #[derive(Component, FromRow)]
//! # struct Foo(u32);
//! # impl PrimaryKey for Foo {
//! #     type Column = u32;
//! #     fn primary_key(&self) -> Self::Column { self.0 }
//! # }
//! # let url = "sqlite:db/sqlite.db";
//! # let mut app = App::new();
//! # app.add_plugins(DefaultPlugins);
//! # app.add_plugins(SqlxPlugin::<Sqlite, Foo>::from_url(&url));
//! fn select(mut events: EventWriter<SqlxEvent<Sqlite, Foo>>) {
//!     let sql = "SELECT * FROM foos";
//!     events.send(SqlxEvent::<Sqlite, Foo>::query_sync(sql));
//! }
//! ```
//!
//! - Notice the effects of [`SqlxTasks::handle_tasks`]
//!
//! ```
//! # use bevy::prelude::*;
//! # use sqlx::{FromRow, Sqlite};
//! # use bevy_sqlx::{SqlxPlugin, SqlxEvent, PrimaryKey};
//! # #[derive(Component, FromRow, Debug)]
//! # struct Foo(u32);
//! # impl PrimaryKey for Foo {
//! #     type Column = u32;
//! #     fn primary_key(&self) -> Self::Column { self.0 }
//! # }
//! # let url = "sqlite:db/sqlite.db";
//! # let mut app = App::new();
//! # app.add_plugins(DefaultPlugins);
//! # app.add_plugins(SqlxPlugin::<Sqlite, Foo>::from_url(&url));
//! fn query(mut foos: Query<&Foo>) {
//!     for foo in &foos {
//!         dbg!(foo);
//!     }
//! }
//! ```
//!
//! - Respond to [`SqlxEventStatus`] events
//!
//! ```
//! # use bevy::prelude::*;
//! # use sqlx::{FromRow, Sqlite};
//! # use bevy_sqlx::{SqlxPlugin, SqlxEvent, SqlxEventStatus, PrimaryKey};
//! # #[derive(Component, FromRow)]
//! # struct Foo(u32);
//! # impl PrimaryKey for Foo {
//! #     type Column = u32;
//! #     fn primary_key(&self) -> Self::Column { self.0 }
//! # }
//! fn status(
//!     mut statuses: EventReader<SqlxEventStatus<Sqlite, Foo>>,
//! ) {
//!     for status in statuses.read() {
//!         match status {
//!             SqlxEventStatus::Start(id) => {},
//!             SqlxEventStatus::Return(id, comp) => {},
//!             SqlxEventStatus::Spawn(id, pk, _) => {},
//!             SqlxEventStatus::Update(id, pk, _) => {},
//!             SqlxEventStatus::Error(id, err) => {},
//!         }
//!     }
//! }
//! ```

pub mod component;
pub use self::component::*;

pub mod event;
pub use self::event::*;
mod database;
pub use self::database::*;

mod plugin;
pub use self::plugin::*;

mod tasks;
pub use self::tasks::*;
