#![allow(clippy::needless_lifetimes)]
#![allow(clippy::bool_assert_comparison)]
#![allow(clippy::too_many_arguments)]
#![deny(unused_must_use)]

#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_derives_extra;
extern crate diesel_derives_traits;
#[macro_use]
extern crate diesel_migrations;
diesel_migrations::embed_migrations!("migrations");

pub mod client;
pub mod filters;
pub mod models;
pub mod schema;
pub mod test_utils;
pub mod utils;

use anyhow::{anyhow, Result};
use diesel::prelude::*;
use diesel::r2d2;
use diesel::r2d2::ConnectionManager;
use diesel::r2d2::Pool;

pub fn run_migrations(conn: &PgConnection) -> Result<()> {
    embedded_migrations::run_with_output(conn, &mut std::io::stdout())?;
    Ok(())
}

/// returns a PgConnection pool
pub fn new_connection_pool(
    database_url: String,
    max_pool_size: u32,
) -> Result<Pool<ConnectionManager<PgConnection>>> {
    if max_pool_size < 1 {
        return Err(anyhow!("max_pool_size less than 1"));
    }
    let manager: ConnectionManager<PgConnection> = ConnectionManager::new(database_url);
    let pool = r2d2::Pool::builder()
        .min_idle(Some(1)) // always keep min idle to 1
        .max_size(max_pool_size)
        .build(manager)?;
    Ok(pool)
}

pub fn establish_connection(database_url: String) -> PgConnection {
    PgConnection::establish(database_url.as_str())
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
}
