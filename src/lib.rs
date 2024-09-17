mod component;
pub use self::component::*;
mod database;
pub use self::database::*;
mod event;
pub use self::event::*;
mod plugin;
pub use self::plugin::*;
mod tasks;
pub use self::tasks::*;

#[cfg(test)]
mod tests {
    use bevy::prelude::*;
    use bevy::tasks::TaskPool;
    use sqlx::{FromRow, Sqlite};
    use crate::*;

    #[derive(Component, FromRow, Debug)]
    struct Foo {
        id: u32,
        text: String,
    }

    impl SqlxPrimaryKey for Foo {
        type Column = u32;
        fn id(&self) -> Self::Column {
            self.id
        }
    }

    fn setup_app() -> App {
        AsyncComputeTaskPool::get_or_init(|| TaskPool::new());
        let url = "sqlite:db/sqlite.db";
        let mut app = App::new();
        app.add_plugins(SqlxPlugin::<Sqlite, Foo>::url(url));
        app
    }

    #[test]
    fn test_query() {
        let mut app = setup_app();
        let mut system_state: SystemState<Query<&Foo>> = SystemState::new(app.world_mut());

        let sql = "INSERT INTO foos (text) VALUES ('test query') RETURNING *";
        let insert = SqlxEvent::<Sqlite, Foo>::query(sql);
        app.world_mut().send_event(insert);

        let mut tries = 0;
        let mut len = system_state.get(app.world()).iter().len();
        while !(len > 0) && tries < 1000 {
            app.update();
            len = system_state.get(app.world()).iter().len();
            tries += 1;
        }

        let query = system_state.get(app.world());
        assert_eq!("test query", query.single().text);
    }

    #[test]
    fn test_callback() {
        let mut app = setup_app();
        let mut system_state: SystemState<Query<&Foo>> = SystemState::new(app.world_mut());

        let delete = SqlxEvent::<Sqlite, Foo>::query("DELETE FROM foos");
        app.world_mut().send_event(delete);

        let text = "test callback";
        let insert = SqlxEvent::<Sqlite, Foo>::call(None, move |db| { async move {
            sqlx::query_as("INSERT INTO foos (text) VALUES (?) RETURNING *")
                .bind(text)
                .fetch_all(&db).await
        }});
        app.world_mut().send_event(insert);

        let mut tries = 0;
        let mut len = system_state.get(app.world()).iter().len();
        while !(len > 0) && tries < 1000 {
            app.update();
            len = system_state.get(app.world()).iter().len();
            tries += 1;
        }

        let query = system_state.get(app.world());
        assert_eq!(text, query.single().text);
    }
}
