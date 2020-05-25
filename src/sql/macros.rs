#[macro_export]
macro_rules! paramsx {
    () => {
        sqlx::sqlite::SqliteArguments::default()
    };
    ($($param:expr),+ $(,)?) => {{
        use sqlx::Arguments;

        let mut args = sqlx::sqlite::SqliteArguments::default();
        $(args.add($param);)+
        args
    }};
}
