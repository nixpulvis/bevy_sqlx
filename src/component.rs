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
use std::hash::Hash;

/// Rows in the database represent a spesifc [`Component`]
pub trait SqlxComponent<R: Row>:
    PrimaryKey + Component + for<'r> FromRow<'r, R> + Unpin
{
}

impl<C, R> SqlxComponent<R> for C
where
    C: PrimaryKey + Component + for<'r> FromRow<'r, R> + Unpin,
    R: Row,
{
}

/// A way to identify components by themselves
///
/// The key is what [`SqlxIndex`](crate::SqlxIndex) maps to entities, so two
/// components with equal keys are the same row.
pub trait PrimaryKey {
    /// The type of the primary key column
    type Key: Clone + Eq + Hash + Send + Sync + 'static;

    /// The name of the primary key column
    const PRIMARY_KEY: &'static str = "id";

    /// The primary key of this component's row
    fn primary_key(&self) -> Self::Key;
}

/// A record that can be upserted into the database
//
// TODO: https://github.com/nixpulvis/bevy_sqlx/issues/7
pub trait ToRow {}

/// An empty [`Component`] for use without a backing table
#[derive(Component, FromRow, Debug, Clone)]
pub struct SqlxDummy {}
impl PrimaryKey for SqlxDummy {
    type Key = ();
    fn primary_key(&self) {}
}
