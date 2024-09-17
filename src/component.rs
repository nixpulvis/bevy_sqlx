//! Components represent data backed by rows in the database
//!
//! Given a table `foos`:
//! | id | text    | flag |
//! | -- | ------- | ---- |
//! | 1  | test    | f    |
//! | 2  | example | f    |
//! | 4  | hello   | t    |
//!
//! A request for `SELECT * FROM foos` would return:
//! ```notest
//! [
//!  { id: 1, text: "test", flag: false }
//!  { id: 2, text: "example", flag: false }
//!  { id: 4, text: "hello", flag: true }
//! ]
//! ```
//!
//! TODO: Explain `ToRow` and `FromRow` here.
use bevy::prelude::*;
use sqlx::{FromRow, Row};

/// Rows in the database represent a spesifc [`Component`]
pub trait SqlxComponent<R: Row>:
    PrimaryKey + Component + for<'r> FromRow<'r, R> + Unpin {}

impl<C, R> SqlxComponent<R> for C
where
    C: PrimaryKey + Component + for<'r> FromRow<'r, R> + Unpin,
    R: Row {}


/// A way to identify components by themselves
//
// TODO: Look into impl PartialEq<PrimaryKey<...>> for Foo
pub trait PrimaryKey {
    type Column: Clone + PartialEq + Send + Sync;
    // fn primary_key_name() -> &'static str;
    fn primary_key(&self) -> Self::Column;
}

/// A record that can be upserted into the database
//
// TODO: https://github.com/nixpulvis/bevy_sqlx/issues/7
pub trait ToRow {}

/// An empty [`Component`] for use without a backing table
#[derive(Component, FromRow, Debug, Clone)]
pub struct SqlxDummy {}
impl PrimaryKey for SqlxDummy {
    type Column = ();
    fn primary_key(&self) {}
}
