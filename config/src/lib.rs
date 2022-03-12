#![allow(clippy::needless_lifetimes)]
#![allow(clippy::bool_assert_comparison)]
#![allow(non_upper_case_globals)]

pub mod analytics;
pub mod database;
pub mod liquidator;
pub mod programs;
pub mod rpcs;
pub mod telemetry;
pub mod utils;

use telemetry::Telemetry;

use crate::{
    analytics::Analytics,
    database::Database,
    programs::Programs,
    rpcs::{RPCEndpoint, RPCs},
};
use anchor_client::solana_sdk::{
    pubkey::Pubkey,
    signature::{read_keypair_file, Keypair},
};

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use simplelog::*;
use solana_clap_utils::keypair::signer_from_path;
use solana_remote_wallet::remote_wallet;
use solana_sdk::signer::Signer;
use std::fs;
use std::{fs::File, str::FromStr};

/// main configuration object
#[remain::sorted]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Configuration {
    pub analytics: Analytics,
    pub database: Database,
    pub debug_log: bool,
    pub key_path: String,
    pub log_file: String,
    pub programs: Programs,
    pub rpc_endpoints: RPCs,
    pub telemetry: Telemetry,
}

impl Programs {
    pub fn lending_id(&self) -> Pubkey {
        Pubkey::from_str(self.lending.id.as_str()).expect("failed to parse vault id")
    }
}
impl Configuration {
    /// cleans the configuration file removing any potentially sensitive information
    /// useful for storing configuration information in version control without accidentally
    /// storing sensitive information
    pub fn sanitize(&mut self) {
        self.key_path = "".to_string();
        self.log_file = "".to_string();
        self.database.conn_url = "".to_string();
        self.rpc_endpoints.primary_endpoint.http_url = "".to_string();
        self.rpc_endpoints.primary_endpoint.ws_url = "".to_string();
        self.rpc_endpoints.failover_endpoints = vec![];
        self.telemetry.agent_endpoint = "".to_string();
    }
    pub fn new_config_file(path: &str, as_json: bool) -> Result<()> {
        let config = Configuration::default();
        config.save(path, as_json)
    }
    pub fn save(&self, path: &str, as_json: bool) -> Result<()> {
        let data = if as_json {
            serde_json::to_string_pretty(&self)?
        } else {
            serde_yaml::to_string(&self)?
        };
        fs::write(path, data).expect("failed to write to file");
        Ok(())
    }
    pub fn load(path: &str, from_json: bool, init_log: bool) -> Result<Configuration> {
        let data = fs::read(path).expect("failed to read file");
        let config: Configuration = if from_json {
            serde_json::from_slice(data.as_slice())?
        } else {
            serde_yaml::from_slice(data.as_slice())?
        };
        if init_log {
            config.init_log(false)?;
        }
        Ok(config)
    }
    /// loads the contents of key_path as a Keypair, does not support hardware wallets
    pub fn payer(&self) -> Keypair {
        read_keypair_file(self.key_path.clone()).expect("failed to read keypair file")
    }
    /// like `payer` but returns a signer instead and can be used with key files or hardware wallets
    pub fn payer_signer(&self, matches: Option<&clap::ArgMatches>) -> Result<Box<dyn Signer>> {
        if self.key_path.starts_with("usb://ledger") {
            self.usb_payer(matches.unwrap())
        } else {
            Ok(Box::new(self.payer()))
        }
    }
    pub fn usb_payer(&self, matches: &clap::ArgMatches) -> Result<Box<dyn Signer>> {
        let mut wallet_manager = remote_wallet::maybe_wallet_manager().unwrap();
        match signer_from_path(matches, &self.key_path, &self.key_path, &mut wallet_manager) {
            Err(err) => {
                return Err(anyhow!(
                    "encountered error retrieving signer from path {:#?}",
                    err
                ));
            }
            Ok(signer) => Ok(signer),
        }
    }

    /// if file_log is true, log to both file and stdout
    /// otherwise just log to stdout
    pub fn init_log(&self, file_log: bool) -> Result<()> {
        if !file_log {
            if self.debug_log {
                TermLogger::init(
                    LevelFilter::Debug,
                    ConfigBuilder::new()
                        .set_location_level(LevelFilter::Debug)
                        .build(),
                    TerminalMode::Mixed,
                    ColorChoice::Auto,
                )?;
            } else {
                TermLogger::init(
                    LevelFilter::Info,
                    ConfigBuilder::new()
                        .set_location_level(LevelFilter::Error)
                        .build(),
                    TerminalMode::Mixed,
                    ColorChoice::Auto,
                )?;
            }
        } else if self.debug_log {
            CombinedLogger::init(vec![
                TermLogger::new(
                    LevelFilter::Debug,
                    ConfigBuilder::new()
                        .set_location_level(LevelFilter::Debug)
                        .build(),
                    TerminalMode::Mixed,
                    ColorChoice::Auto,
                ),
                WriteLogger::new(
                    LevelFilter::Debug,
                    ConfigBuilder::new()
                        .set_location_level(LevelFilter::Debug)
                        .build(),
                    File::create(self.log_file.as_str()).unwrap(),
                ),
            ])?;
        } else {
            CombinedLogger::init(vec![
                TermLogger::new(
                    LevelFilter::Info,
                    ConfigBuilder::new()
                        .set_location_level(LevelFilter::Error)
                        .build(),
                    TerminalMode::Mixed,
                    ColorChoice::Auto,
                ),
                WriteLogger::new(
                    LevelFilter::Info,
                    ConfigBuilder::new()
                        .set_location_level(LevelFilter::Error)
                        .build(),
                    File::create(self.log_file.as_str()).unwrap(),
                ),
            ])?;
        }
        Ok(())
    }
}

impl Default for Configuration {
    fn default() -> Self {
        Configuration {
            rpc_endpoints: RPCs {
                primary_endpoint: RPCEndpoint {
                    http_url: "https://api.mainnet-beta.solana.com".to_string(),
                    ws_url: "ws://api.mainnet-beta.solana.com".to_string(),
                },
                failover_endpoints: vec![RPCEndpoint {
                    http_url: "https://solana-api.projectserum.com".to_string(),
                    ws_url: "ws://solana-api.projectserum.com".to_string(),
                }],
            },
            key_path: "".to_string(),
            database: Database {
                analytics_pool_size: 2,
                conn_url: "postgres://postgres:password123@localhost/liquidator".to_string(),
                pool_size: 2,
            },
            programs: Programs::default(),
            log_file: "".to_string(),
            analytics: Analytics::default(),
            debug_log: false,
            telemetry: Telemetry {
                enabled: true,
                agent_endpoint: String::from("http://localhost:8126"),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_sanitize() {
        let mut config = Configuration::default();
        config.key_path = "420".to_string();
        config.database.conn_url = "420".to_string();
        config.rpc_endpoints.primary_endpoint.http_url = "420".to_string();
        config.rpc_endpoints.primary_endpoint.ws_url = "420".to_string();
        config.rpc_endpoints.failover_endpoints.push(RPCEndpoint {
            http_url: "420".to_string(),
            ws_url: "420".to_string(),
        });
        config.sanitize();
        assert!(config.key_path.is_empty());
        assert!(config.database.conn_url.is_empty());
        assert!(config.rpc_endpoints.primary_endpoint.http_url.is_empty());
        assert!(config.rpc_endpoints.primary_endpoint.ws_url.is_empty());
        assert!(config.rpc_endpoints.failover_endpoints.is_empty());
    }
}
