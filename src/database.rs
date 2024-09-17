use bevy::prelude::*;
use sqlx::{Database, Pool};

#[derive(Resource, Debug)]
pub struct SqlxDatabase<DB: Database> {
    pub pool: Pool<DB>,
}
