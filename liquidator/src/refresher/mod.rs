//! service responsible for ensuring all obligations stored in the database
//! have their ltv values periodically refreshed
use std::sync::Arc;
pub mod refresh;
use anchor_lang::prelude::*;
use anyhow::{anyhow, Result};
use chrono::Utc;
use config::Configuration;
use crossbeam::select;
use crossbeam_channel::tick;
use diesel::r2d2;
use diesel::PgConnection;
use log::{error, info, warn};
use rayon::ThreadPoolBuilder;
use solana_account_decoder::UiAccountEncoding;
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use std::str::FromStr;

use db::filters::{LtvFilter, ObligationMatcher};

use crate::refresher::refresh::handle_pseudo_obligation_refresh;

pub struct Refresher {
    pub cfg: Arc<Configuration>,
    pub pool: r2d2::Pool<r2d2::ConnectionManager<PgConnection>>,
    pub rpc: Arc<RpcClient>,
}

impl Refresher {
    pub fn new(cfg: Arc<Configuration>) -> Result<Arc<Refresher>> {
        let pool = db::new_connection_pool(
            cfg.database.conn_url.clone(),
            cfg.refresher.pool_size as u32,
        )?;
        let rpc = cfg.get_rpc_client(false, None);
        Ok(Arc::new(Refresher {
            cfg,
            pool,
            rpc: Arc::new(rpc),
        }))
    }
    pub fn start(self: &Arc<Self>, exit_chan: crossbeam_channel::Receiver<bool>) -> Result<()> {
        let pool = ThreadPoolBuilder::new()
            .num_threads(self.cfg.refresher.max_concurrency as usize)
            .build()?;
        let ticker = tick(std::time::Duration::from_secs(self.cfg.refresher.frequency));
        loop {
            select! {
                recv(ticker) -> _msg => {
                    let obligations = {
                        let conn = match self.pool.get() {
                            Ok(conn) => conn,
                            Err(err) => {
                                error!("failed to retrieve connection pool {:#?}", err);
                                continue;
                            }
                        };
                        match db::client::get_obligation(
                            &conn,
                            &ObligationMatcher::All,
                            None
                        ) {
                            Ok(obligations) => obligations,
                            Err(err) => {
                                error!("failed to retrieve obligations {:#?}", err);
                                continue;
                            }
                        }
                    };
                    for obligation in obligations {
                        let rpc = Arc::clone(&self.rpc);
                        let obligation = obligation.clone();
                        let conn = match self.pool.get() {
                            Ok(conn) => conn,
                            Err(err) => {
                                error!("failed to retrieve connection pool {:#?}", err);
                                continue;
                            }
                        };
                        pool.spawn(move || {
                            let lending_obligation = match handle_pseudo_obligation_refresh(&rpc, &obligation) {
                                Ok(obligation) => obligation,
                                Err(err) => {
                                    error!("failed to pseudo refresh obligation {:#?}", err);
                                    return;
                                }
                            };
                            let ltv = match lending_obligation.loan_to_value() {
                                Ok(ltv) => ltv,
                                Err(err) => {
                                    error!("failed to calculate ltv {:#?}", err);
                                    return;
                                }
                            };
                            let ltv = match f64::from_str(&ltv.to_string()) {
                                Ok(ltv) => ltv,
                                Err(err) => {
                                    error!("failed to parse ltv to float {:#?}", err);
                                    return;
                                }
                            };
                            match db::client::put_obligation(&conn, ltv, &obligation.account, Utc::now()) {
                                Ok(_) => (),
                                Err(err) => error!("failed to update obligation ltv {:#?}", err),
                            }
                        });

                    }
                }
            }
        }
    }
}
