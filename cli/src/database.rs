use config::Configuration;

use anyhow::{anyhow, Result};

#[cfg(not(tarpaulin_include))]
pub fn run_database_migrations(config_file_path: String) -> Result<()> {
    let config = Configuration::load(config_file_path.as_str(), false, true)
        .expect("failed to load config file");
    let conn = db::establish_connection(config.database.conn_url);
    let err = db::run_migrations(&conn);
    if err.is_err() {
        return Err(anyhow!(
            "failed to run migrations {:#?}",
            err.err().unwrap()
        ));
    }
    Ok(())
}
