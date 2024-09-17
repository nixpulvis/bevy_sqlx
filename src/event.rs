use bevy::prelude::*;
use sqlx::{Database, Error, Executor, IntoArguments, Pool};
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::Arc;
use crate::*;

type SqlxEventFunc<DB, C> = Arc<dyn Fn(Pool<DB>) ->
    Pin<Box<dyn Future<Output = Result<Vec<C>, Error>> + Send>> + Send + Sync>;

#[derive(Event, Clone)]
pub struct SqlxEvent<DB: Database, C: SqlxComponent<DB::Row>> {
    label: Option<String>,
    func: SqlxEventFunc<DB, C>,
    _db: PhantomData<DB>,
    _c: PhantomData<C>,
}

impl<DB: Database + Sync, C: SqlxComponent<DB::Row>> SqlxEvent<DB, C>
where
    for<'c> &'c mut DB::Connection: Executor<'c, Database = DB>,
    for<'a> <DB as sqlx::Database>::Arguments<'a>: IntoArguments<'a, DB>,
{
    pub fn query(string: &str) -> Self {
        let arc: Arc<str> = string.into();
        Self::call(Some(string), move |db| {
            let s = arc.clone();
            async move {
                sqlx::query_as(&s).fetch_all(&db).await
            }
        })
    }

    pub fn call<F, T>(label: Option<&str>, func: F) -> Self
    where
        F: Fn(Pool<DB>) -> T + Send + Sync + 'static,
        T: Future<Output = Result<Vec<C>, Error>> + Send + 'static,
    {
        SqlxEvent {
            label: label.map(|s| s.to_string()),
            func: Arc::new(move |db: Pool<DB>| {
                Box::pin(func(db))
            }),
            _db: PhantomData::<DB>,
            _c: PhantomData::<C>,
        }
    }

    pub fn send(self, events: &mut EventWriter<SqlxEvent<DB, C>>) -> Self {
        events.send(SqlxEvent {
            label: self.label.clone(),
            func: self.func.clone(),
            _db: PhantomData::<DB>,
            _c: PhantomData::<C>,
        });
        self
    }

    pub fn trigger(self, commands: &mut Commands) -> Self {
        commands.trigger(SqlxEvent {
            label: self.label.clone(),
            func: self.func.clone(),
            _db: PhantomData::<DB>,
            _c: PhantomData::<C>,
        });
        self
    }

    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    pub(crate) fn func(&self) -> &SqlxEventFunc<DB, C> {
        &self.func
    }
}
