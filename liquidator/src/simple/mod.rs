//! the simple liquidator with no support for flashloans

pub mod handler;

use anyhow::{anyhow, Result};
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

use db::filters::{LtvFilter, ObligationMatcher};
use std::sync::Arc;

pub struct Obligation {
    pub ltv: f64,
    pub account: String,
}

pub struct SimpleLiquidator {
    pub cfg: Arc<Configuration>,
    pub pool: r2d2::Pool<r2d2::ConnectionManager<PgConnection>>,
    pub rpc: Arc<RpcClient>,
}

impl SimpleLiquidator {
    pub fn new(cfg: Arc<Configuration>) -> Result<Arc<SimpleLiquidator>> {
        let pool = db::new_connection_pool(cfg.database.conn_url.clone(), cfg.database.pool_size)?;
        let rpc = cfg.get_rpc_client(false, None);
        Ok(Arc::new(SimpleLiquidator {
            cfg,
            pool,
            rpc: Arc::new(rpc),
        }))
    }
    pub fn start(
        self: &Arc<Self>,
        ltv_filter: LtvFilter,
        exit_chan: crossbeam_channel::Receiver<bool>,
    ) -> Result<()> {
        let pool = ThreadPoolBuilder::new()
            .num_threads(self.cfg.liquidator.max_concurrency as usize)
            .build()?;
        let conn = self.pool.get()?;
        let ticker = tick(std::time::Duration::from_secs(
            self.cfg.liquidator.frequency,
        ));
        loop {
            select! {
                recv(ticker) -> _msg => {
                    let mut obligations = match db::client::get_obligation(
                        &conn,
                        &ObligationMatcher::All,
                        Some(ltv_filter)
                    ) {
                        Ok(obligations) => obligations,
                        Err(err) => {
                            error!("failed to retrieve obligations {:#?}", err);
                            continue;
                        }
                    };
                    
                    // sort obligations by ltv so we're liquidating the most 
                    // unhealthy positions first
                    obligations.sort_unstable_by_key(|obligation| obligation.ltv);

                    for obligation in obligations {
                        let service = Arc::clone(self);
                        let obligation = obligation.clone();
                        pool.spawn(move || {
                            match service.handle_liquidation_check(&obligation) {
                                Ok(_) => (),
                                Err(err) => error!(
                                    "liquidation for obligation {} failed: {:#?}",
                                    obligation.account, err,
                                ),
                            };
                        });
                    }
                }
                recv(exit_chan) -> _msg => {
                    warn!("work_queue filler received exit notification");
                    return Ok(());
                }
            }
        }
    }
}
