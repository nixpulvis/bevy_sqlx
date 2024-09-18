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

// /// Rows in the database represent a spesifc [`Component`]
// pub trait SqlxComponent<R: Row>:
//     Component + for<'r> FromRow<'r, R> + Unpin
// {
//     type Column: Clone + PartialEq + Send + Sync;
//     // fn primary_key_name() -> &'static str;
//     fn primary_key(&self) -> Self::Column;
// }

/// Rows in the database represent a spesifc [`Component`]
pub trait SqlxComponent<R: Row>:
    PrimaryKey + Component + for<'r> FromRow<'r, R> + Unpin
{
}

pub trait PrimaryKey {
    type Column: Send + Sync + PartialEq;

    fn primary_key(&self) -> Self::Column;
}

impl PrimaryKey for () {
    type Column = Self;

    fn primary_key(&self) -> Self {
        *self
    }
}

/// A record that can be upserted into the database
//
// TODO: https://github.com/nixpulvis/bevy_sqlx/issues/7
pub trait ToRow {}
