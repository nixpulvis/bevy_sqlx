use bevy::prelude::*;
use bevy::tasks::Task;
use sqlx::{Error, Row};
use std::marker::PhantomData;
use crate::*;

#[derive(Resource, Debug)]
pub struct SqlxTasks<R: Row, C: SqlxComponent<R>> {
    pub components: Vec<Task<Result<Vec<C>, Error>>>,
    _r: PhantomData<R>,
}

impl<R: Row, C: SqlxComponent<R>> Default for SqlxTasks<R, C> {
    fn default() -> Self {
        SqlxTasks {
            components: Vec::new(),
            _r: PhantomData::<R>,
        }
    }
}
