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
    PrimaryKey + Component + ToRow + for<'r> FromRow<'r, R> + Unpin
{
}

impl<
        R: Row,
        C: PrimaryKey + Component + ToRow + for<'r> FromRow<'r, R> + Unpin,
    > SqlxComponent<R> for C
{
}

pub trait PrimaryKey {
    fn primary_key(&self) -> SqlxColumn;
}

impl PrimaryKey for () {
    fn primary_key(&self) -> SqlxColumn {
        SqlxColumn::new("id", "")
    }
}

/// A record that can be upserted into the database
//
// TODO: https://github.com/nixpulvis/bevy_sqlx/issues/7
pub trait ToRow {
    fn to_row(&self) -> Vec<SqlxColumn>;
}

#[derive(Debug, PartialEq)]
pub struct SqlxColumn {
    name: String,
    value: String,
}

impl SqlxColumn {
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        SqlxColumn { name: name.into(), value: value.into() }
    }
}

pub trait SqlxColumns {
    fn sql_names(&self) -> String;
    fn sql_values(&self) -> String;
}

impl SqlxColumns for Vec<SqlxColumn> {
    fn sql_names(&self) -> String {
        self.iter().map(|c| c.name.clone()).collect::<Vec<_>>().join(", ")
    }

    fn sql_values(&self) -> String {
        self.iter().map(|c| c.value.to_string()).collect::<Vec<_>>().join(", ")
    }
}
