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
use sqlx::{Database, FromRow, Row};
use std::marker::PhantomData;

/// Rows in the database represent a spesifc [`Component`]
pub trait SqlxComponent<R: Row>:
    Component + for<'r> FromRow<'r, R> + Unpin
{
    type Column: Clone + PartialEq + Send + Sync;
    // fn primary_key_name() -> &'static str;
    fn primary_key(&self) -> Self::Column;
}

// impl<C, R> SqlxComponent<R> for C
// where
//     C: Component + for<'r> FromRow<'r, R> + Unpin,
//     R: Row,
// {
//     type Column = u32;
//     // fn primary_key_name() -> &'static str;
//     fn primary_key(&self) -> Self::Column {
//         unimplemented!()
//     }
// }

/// A record that can be upserted into the database
//
// TODO: https://github.com/nixpulvis/bevy_sqlx/issues/7
pub trait ToRow {}

/// An empty [`Component`] for use without a backing table
#[derive(Component, FromRow, Debug, Clone)]
pub struct SqlxDummy<DB: Database, Q: DB::Row>(PhantomData<Q>, PhantomData<DB>);

impl<DB: Database, R: DB::Row> SqlxComponent<R> for SqlxDummy<DB> {
    type Column = ();
    fn primary_key(&self) -> () {}
}
