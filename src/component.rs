use bevy::prelude::*;
use sqlx::{FromRow, Row};

pub trait SqlxComponent<R: Row>:
    SqlxPrimaryKey + Component + for<'r> FromRow<'r, R> + Unpin
{
}
impl<C, R> SqlxComponent<R> for C
where
    C: SqlxPrimaryKey + Component + for<'r> FromRow<'r, R> + Unpin,
    R: Row,
{
}

pub trait SqlxPrimaryKey {
    type Column: PartialEq;
    fn id(&self) -> Self::Column;
}

#[derive(Component, FromRow)]
pub struct SqlxDummy {}
impl SqlxPrimaryKey for SqlxDummy {
    type Column = ();
    fn id(&self) {}
}
