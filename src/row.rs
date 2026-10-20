//! Writing components back to the database
//!
//! [`ToRow`] is the inverse of [`FromRow`](sqlx::FromRow): it names a table
//! and binds each column as a typed value. The crate builds the SQL text from
//! those names and lets the database's own [`Arguments`] choose the
//! placeholder style, so values are never formatted into SQL.
//!
//! ```
//! # use sqlx::{Database, Encode, Type};
//! # use sqlx::error::BoxDynError;
//! # use bevy_sqlx::{ToRow, RowWriter};
//! struct Foo {
//!     id: i64,
//!     text: String,
//!     flag: bool,
//! }
//!
//! impl<DB: Database> ToRow<DB> for Foo
//! where
//!     i64: for<'q> Encode<'q, DB> + Type<DB>,
//!     String: for<'q> Encode<'q, DB> + Type<DB>,
//!     bool: for<'q> Encode<'q, DB> + Type<DB>,
//! {
//!     const TABLE: &'static str = "foos";
//!
//!     fn write_columns(&self, row: &mut RowWriter<DB>) {
//!         row.bind("text", self.text.clone());
//!         row.bind("flag", self.flag);
//!     }
//!
//!     fn write_key(&self, row: &mut RowWriter<DB>) {
//!         row.bind("id", self.id);
//!     }
//! }
//! ```
use sqlx::error::BoxDynError;
use sqlx::{Arguments, Database, Encode, Type};
use std::fmt::Write;

/// A record which can be written to a table in the database
pub trait ToRow<DB: Database> {
    /// The table rows of this type live in
    const TABLE: &'static str;

    /// Bind every non-key column, in a stable order
    fn write_columns(&self, row: &mut RowWriter<DB>);

    /// Bind the primary key column(s)
    ///
    /// Kept apart from [`Self::write_columns`] so `UPDATE` and `DELETE` can put
    /// the key in their `WHERE` clause, and `INSERT` can leave it out for the
    /// database to generate.
    fn write_key(&self, row: &mut RowWriter<DB>);
}

/// Column names and their values, collected from a [`ToRow`]
///
/// Values are held until the statement runs, then bound into the database's
/// [`Arguments`]. sqlx 0.8 ties `Arguments<'q>` to the lifetime of the SQL
/// string, so the arguments can only be created next to the SQL inside the
/// task that runs it.
pub struct RowWriter<DB: Database> {
    columns: Vec<(&'static str, Box<dyn Bind<DB>>)>,
}

impl<DB: Database> Default for RowWriter<DB> {
    fn default() -> Self {
        RowWriter { columns: Vec::new() }
    }
}

impl<DB: Database> RowWriter<DB> {
    /// Bind `value` to the column `name`
    pub fn bind<T>(&mut self, name: &'static str, value: T)
    where
        T: for<'q> Encode<'q, DB> + Type<DB> + Send + Sync + 'static,
    {
        self.columns.push((name, Box::new(Value(value))));
    }

    /// The column names bound so far, in order
    pub fn names(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.columns.iter().map(|(name, _)| *name)
    }

    fn is_empty(&self) -> bool {
        self.columns.is_empty()
    }
}

trait Bind<DB: Database>: Send + Sync {
    fn bind<'q>(&'q self, args: &mut DB::Arguments<'q>, sql: &mut String) -> Result<(), BoxDynError>;
}

struct Value<T>(T);

impl<DB, T> Bind<DB> for Value<T>
where
    DB: Database,
    T: for<'q> Encode<'q, DB> + Type<DB> + Send + Sync,
{
    fn bind<'q>(&'q self, args: &mut DB::Arguments<'q>, sql: &mut String) -> Result<(), BoxDynError> {
        args.add(&self.0)?;
        args.format_placeholder(sql)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Insert,
    Update,
    Delete,
}

/// A single `INSERT`, `UPDATE` or `DELETE` built from a [`ToRow`]
///
/// The values are owned, so a statement can be moved into a task and run any
/// number of times.
pub struct RowStatement<DB: Database> {
    kind: Kind,
    table: &'static str,
    columns: RowWriter<DB>,
    key: RowWriter<DB>,
}

impl<DB: Database> RowStatement<DB> {
    /// `INSERT INTO table (columns) VALUES (...) RETURNING *`
    ///
    /// The key is left out so the database can generate it.
    pub fn insert<C: ToRow<DB>>(component: &C) -> Self {
        let mut columns = RowWriter::default();
        component.write_columns(&mut columns);
        RowStatement { kind: Kind::Insert, table: C::TABLE, columns, key: RowWriter::default() }
    }

    /// `UPDATE table SET columns WHERE key RETURNING *`
    pub fn update<C: ToRow<DB>>(component: &C) -> Self {
        let mut columns = RowWriter::default();
        let mut key = RowWriter::default();
        component.write_columns(&mut columns);
        component.write_key(&mut key);
        RowStatement { kind: Kind::Update, table: C::TABLE, columns, key }
    }

    /// `DELETE FROM table WHERE key RETURNING *`
    pub fn delete<C: ToRow<DB>>(component: &C) -> Self {
        let mut key = RowWriter::default();
        component.write_key(&mut key);
        RowStatement { kind: Kind::Delete, table: C::TABLE, columns: RowWriter::default(), key }
    }

    /// Build the SQL text and its arguments
    ///
    /// The SQL is written in the same order the values are bound, which
    /// `?` placeholders require.
    pub fn build(&self) -> Result<(String, DB::Arguments<'_>), BoxDynError> {
        let mut args = DB::Arguments::default();
        let mut sql = String::new();
        let table = quote(self.table);
        match self.kind {
            Kind::Insert if self.columns.is_empty() => {
                write!(sql, "INSERT INTO {table} DEFAULT VALUES")?;
            }
            Kind::Insert => {
                let names = self.columns.names().map(quote).collect::<Vec<_>>().join(", ");
                write!(sql, "INSERT INTO {table} ({names}) VALUES (")?;
                write_list(&self.columns, ", ", &mut args, &mut sql, false)?;
                sql.push(')');
            }
            Kind::Update => {
                if self.columns.is_empty() {
                    return Err("UPDATE needs at least one non-key column".into());
                }
                write!(sql, "UPDATE {table} SET ")?;
                write_list(&self.columns, ", ", &mut args, &mut sql, true)?;
                write_where(&self.key, &mut args, &mut sql)?;
            }
            Kind::Delete => {
                write!(sql, "DELETE FROM {table}")?;
                write_where(&self.key, &mut args, &mut sql)?;
            }
        }
        sql.push_str(" RETURNING *");
        Ok((sql, args))
    }
}

fn write_list<'q, DB: Database>(
    row: &'q RowWriter<DB>,
    sep: &str,
    args: &mut DB::Arguments<'q>,
    sql: &mut String,
    named: bool,
) -> Result<(), BoxDynError> {
    for (i, (name, value)) in row.columns.iter().enumerate() {
        if i > 0 {
            sql.push_str(sep);
        }
        if named {
            write!(sql, "{} = ", quote(name))?;
        }
        value.bind(args, sql)?;
    }
    Ok(())
}

fn write_where<'q, DB: Database>(
    key: &'q RowWriter<DB>,
    args: &mut DB::Arguments<'q>,
    sql: &mut String,
) -> Result<(), BoxDynError> {
    if key.is_empty() {
        return Err("a WHERE clause needs at least one key column".into());
    }
    sql.push_str(" WHERE ");
    write_list(key, " AND ", args, sql, true)
}

/// Quote an identifier the way SQLite and Postgres both accept
fn quote(ident: &str) -> String {
    format!("\"{}\"", ident.replace('"', "\"\""))
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::tasks::block_on;
    use sqlx::pool::PoolOptions;
    use sqlx::{Executor, FromRow, IntoArguments, Pool, Postgres, Sqlite};

    #[derive(FromRow, Debug, Clone, PartialEq)]
    struct Foo {
        id: i64,
        text: String,
        flag: bool,
        note: Option<String>,
        order: i64,
    }

    impl<DB: Database> ToRow<DB> for Foo
    where
        i64: for<'q> Encode<'q, DB> + Type<DB>,
        String: for<'q> Encode<'q, DB> + Type<DB>,
        bool: for<'q> Encode<'q, DB> + Type<DB>,
        Option<String>: for<'q> Encode<'q, DB> + Type<DB>,
    {
        const TABLE: &'static str = "torow_foos";

        fn write_columns(&self, row: &mut RowWriter<DB>) {
            row.bind("text", self.text.clone());
            row.bind("flag", self.flag);
            row.bind("note", self.note.clone());
            row.bind("order", self.order);
        }

        fn write_key(&self, row: &mut RowWriter<DB>) {
            row.bind("id", self.id);
        }
    }

    fn foo() -> Foo {
        Foo { id: 0, text: "it's \"quoted\"".into(), flag: true, note: None, order: 7 }
    }

    fn sql<DB: Database>(statement: RowStatement<DB>) -> String {
        statement.build().unwrap().0
    }

    #[test]
    fn sqlite_sql() {
        let f = foo();
        assert_eq!(
            sql::<Sqlite>(RowStatement::insert(&f)),
            r#"INSERT INTO "torow_foos" ("text", "flag", "note", "order") VALUES (?, ?, ?, ?) RETURNING *"#
        );
        assert_eq!(
            sql::<Sqlite>(RowStatement::update(&f)),
            r#"UPDATE "torow_foos" SET "text" = ?, "flag" = ?, "note" = ?, "order" = ? WHERE "id" = ? RETURNING *"#
        );
        assert_eq!(
            sql::<Sqlite>(RowStatement::delete(&f)),
            r#"DELETE FROM "torow_foos" WHERE "id" = ? RETURNING *"#
        );
    }

    #[test]
    fn postgres_sql() {
        let f = foo();
        assert_eq!(
            sql::<Postgres>(RowStatement::insert(&f)),
            r#"INSERT INTO "torow_foos" ("text", "flag", "note", "order") VALUES ($1, $2, $3, $4) RETURNING *"#
        );
        assert_eq!(
            sql::<Postgres>(RowStatement::update(&f)),
            r#"UPDATE "torow_foos" SET "text" = $1, "flag" = $2, "note" = $3, "order" = $4 WHERE "id" = $5 RETURNING *"#
        );
        assert_eq!(
            sql::<Postgres>(RowStatement::delete(&f)),
            r#"DELETE FROM "torow_foos" WHERE "id" = $1 RETURNING *"#
        );
    }

    async fn run<DB: Database>(pool: &Pool<DB>, statement: RowStatement<DB>) -> Vec<Foo>
    where
        for<'c> &'c Pool<DB>: Executor<'c, Database = DB>,
        for<'q> DB::Arguments<'q>: IntoArguments<'q, DB>,
        for<'r> Foo: FromRow<'r, DB::Row>,
    {
        let (sql, args) = statement.build().unwrap();
        sqlx::query_as_with(&sql, args).fetch_all(pool).await.unwrap()
    }

    async fn round_trip<DB: Database>(pool: Pool<DB>)
    where
        for<'c> &'c Pool<DB>: Executor<'c, Database = DB>,
        for<'q> DB::Arguments<'q>: IntoArguments<'q, DB>,
        for<'r> Foo: FromRow<'r, DB::Row>,
        Foo: ToRow<DB>,
    {
        let mut f = foo();
        let inserted = run(&pool, RowStatement::insert(&f)).await;
        assert_eq!(inserted.len(), 1);
        assert_ne!(inserted[0].id, 0, "database generated the key");
        f.id = inserted[0].id;
        assert_eq!(inserted[0], f);

        f.note = Some("now set".into());
        f.flag = false;
        assert_eq!(run(&pool, RowStatement::update(&f)).await, vec![f.clone()]);

        f.note = None;
        assert_eq!(run(&pool, RowStatement::update(&f)).await, vec![f.clone()]);

        assert_eq!(run(&pool, RowStatement::delete(&f)).await, vec![f.clone()]);
        assert_eq!(run(&pool, RowStatement::delete(&f)).await, vec![]);
    }

    #[test]
    fn sqlite_round_trip() {
        block_on(async {
            let pool = PoolOptions::<Sqlite>::new()
                .max_connections(1)
                .connect("sqlite::memory:")
                .await
                .unwrap();
            pool.execute(
                r#"CREATE TABLE torow_foos (
                    id INTEGER PRIMARY KEY,
                    text TEXT NOT NULL,
                    flag BOOLEAN NOT NULL,
                    note TEXT,
                    "order" INTEGER NOT NULL
                )"#,
            )
            .await
            .unwrap();
            round_trip(pool).await;
        });
    }

    /// Runs when `BEVY_SQLX_TEST_POSTGRES` holds a database URL
    #[test]
    fn postgres_round_trip() {
        let Ok(url) = std::env::var("BEVY_SQLX_TEST_POSTGRES") else {
            return;
        };
        block_on(async {
            let pool = Pool::<Postgres>::connect(&url).await.unwrap();
            pool.execute("DROP TABLE IF EXISTS torow_foos").await.unwrap();
            pool.execute(
                r#"CREATE TABLE torow_foos (
                    id BIGSERIAL PRIMARY KEY,
                    text TEXT NOT NULL,
                    flag BOOLEAN NOT NULL,
                    note TEXT,
                    "order" BIGINT NOT NULL
                )"#,
            )
            .await
            .unwrap();
            round_trip(pool).await;
        });
    }

    mod bevy_path {
        use super::*;
        use crate::{PrimaryKey, SqlxEvent, SqlxEventStatus, SqlxPlugin};
        use bevy::prelude::*;
        use bevy::tasks::{AsyncComputeTaskPool, TaskPool};

        #[derive(Component, FromRow, Debug, Clone, PartialEq)]
        struct Bar {
            id: i64,
            text: String,
        }

        impl PrimaryKey for Bar {
            type Column = i64;
            fn primary_key(&self) -> i64 {
                self.id
            }
        }

        impl ToRow<Sqlite> for Bar {
            const TABLE: &'static str = "bars";
            fn write_columns(&self, row: &mut RowWriter<Sqlite>) {
                row.bind("text", self.text.clone());
            }
            fn write_key(&self, row: &mut RowWriter<Sqlite>) {
                row.bind("id", self.id);
            }
        }

        fn bars(app: &mut App) -> Vec<Bar> {
            let world = app.world_mut();
            world.query::<&Bar>().iter(world).cloned().collect()
        }

        fn update_until(app: &mut App, done: impl Fn(&[Bar]) -> bool) -> Vec<Bar> {
            for _ in 0..1000 {
                app.update();
                let bars = bars(app);
                if done(&bars) {
                    return bars;
                }
            }
            panic!("timed out, have {:?}", bars(app));
        }

        #[test]
        fn insert_then_update_sync() {
            AsyncComputeTaskPool::get_or_init(TaskPool::new);
            let pool = block_on(async {
                let pool = PoolOptions::<Sqlite>::new()
                    .max_connections(1)
                    .connect("sqlite::memory:")
                    .await
                    .unwrap();
                pool.execute("CREATE TABLE bars (id INTEGER PRIMARY KEY, text TEXT NOT NULL)")
                    .await
                    .unwrap();
                pool
            });
            let mut app = App::new();
            app.add_plugins(SqlxPlugin::<Sqlite, Bar>::from_pool(pool));

            let new = Bar { id: 0, text: "new".into() };
            app.world_mut().send_event(SqlxEvent::<Sqlite, Bar>::insert_sync(&new));
            let spawned = update_until(&mut app, |b| !b.is_empty());
            assert_eq!(spawned, vec![Bar { id: 1, text: "new".into() }]);

            let changed = Bar { id: 1, text: "changed".into() };
            app.world_mut().send_event(SqlxEvent::<Sqlite, Bar>::update_sync(&changed));
            let updated = update_until(&mut app, |b| b[0].text == "changed");
            assert_eq!(updated, vec![changed.clone()]);

            app.world_mut().send_event(SqlxEvent::<Sqlite, Bar>::delete(&changed));
            app.world_mut().resource_mut::<Events<SqlxEventStatus<Sqlite, Bar>>>().clear();
            let mut deleted = None;
            for _ in 0..1000 {
                app.update();
                let statuses = app.world().resource::<Events<SqlxEventStatus<Sqlite, Bar>>>();
                let returned = statuses.iter_current_update_events().find_map(|s| match s {
                    SqlxEventStatus::Return(_, rows) => Some(rows.clone()),
                    _ => None,
                });
                if returned.is_some() {
                    deleted = returned;
                    break;
                }
            }
            assert_eq!(deleted, Some(vec![changed]));
        }
    }
}
