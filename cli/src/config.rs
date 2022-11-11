use crate::helpers::get_config;

use anyhow::Result;
use config::Configuration;

#[cfg(not(tarpaulin_include))]
pub fn new_config(_matches: &clap::ArgMatches, config_file_path: String) -> Result<()> {
    Configuration::new_config_file(config_file_path.as_str(), false)?;
    Ok(())
}

#[cfg(not(tarpaulin_include))]
pub fn sanitize(_matches: &clap::ArgMatches, config_file_path: String) -> Result<()> {
    let mut config = get_config(&config_file_path)?;
    config.sanitize();
    let name_parts: Vec<&str> = config_file_path.split('.').collect();
    let mut name = String::new();
    name.push_str(name_parts[0]);
    name.push_str("_sanitized.yaml");
    config.save(name.as_str(), false)?;

    Ok(())
}

#[cfg(not(tarpaulin_include))]
pub fn export_as_json(_matches: &clap::ArgMatches, config_file_path: String) -> Result<()> {
    let mut config = get_config(&config_file_path)?;
    // remove sensitive information
    config.sanitize();
    // update symbol names & farm_types
    let name_parts: Vec<&str> = config_file_path.split('.').collect();

    // save the exported file in json format
    let mut name = String::new();
    name.push_str(name_parts[0]);
    name.push_str(".json");
    config.save(name.as_str(), true)?;
    Ok(())
}
