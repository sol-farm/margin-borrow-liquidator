//! provides a helper object for creating temporary test databases
//! taken from https://github.com/diesel-rs/diesel/issues/1549

use diesel::{sql_query, Connection, PgConnection, RunQueryDsl};
use log::warn;
use std::sync::atomic::AtomicU32;
use url::Url;

static TEST_DB_COUNTER: AtomicU32 = AtomicU32::new(0);

pub struct TestDb {
    default_db_url: String,
    url: String,
    name: String,
    delete_on_drop: bool,
}
impl TestDb {
    pub fn new() -> Self {
        let name = format!(
            "test_db_{}_{}",
            std::process::id(),
            TEST_DB_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
        );
        // todo(bonedaddy): this is temporary
        std::env::set_var(
            "DATABASE_URL",
            "postgres://postgres:password123@localhost/tulip",
        );
        let default_db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL");
        let conn = diesel_logger::LoggingConnection::new(
            PgConnection::establish(&default_db_url).unwrap(),
        );
        sql_query(format!("CREATE DATABASE {};", name))
            .execute(&conn)
            .unwrap();
        let mut url = Url::parse(&default_db_url).unwrap();
        url.set_path(&name);
        Self {
            default_db_url,
            url: url.to_string(),
            name,
            delete_on_drop: true,
        }
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn conn(&self) -> diesel_logger::LoggingConnection<PgConnection> {
        diesel_logger::LoggingConnection::new(PgConnection::establish(self.url.as_str()).unwrap())
    }

    pub fn leak(&mut self) {
        self.delete_on_drop = false;
    }
}
impl Drop for TestDb {
    fn drop(&mut self) {
        if !self.delete_on_drop {
            warn!("TestDb leaking database {}", self.name);
            return;
        }
        let conn = diesel_logger::LoggingConnection::new(
            PgConnection::establish(&self.default_db_url).unwrap(),
        );
        sql_query(format!(
            "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = '{}'",
            self.name
        ))
        .execute(&conn)
        .unwrap();
        sql_query(format!("DROP DATABASE {}", self.name))
            .execute(&conn)
            .unwrap();
    }
}

impl Default for TestDb {
    fn default() -> Self {
        Self::new()
    }
}
