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
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;

use std::sync::Arc;

pub struct Obligation {
    pub ltv: f64,
    pub account: String,
}

pub struct SimpleLiquidator {
    pub cfg: Arc<Configuration>,
    pub db: Arc<db::LiquidatorDb>,
    pub rpc: Arc<RpcClient>,
}

impl SimpleLiquidator {
    pub fn new(cfg: Configuration) -> Result<Arc<SimpleLiquidator>> {
        let db = db::LiquidatorDb::new(cfg.clone())?;
        let rpc = cfg.get_rpc_client(false, None);
        Ok(Arc::new(SimpleLiquidator {
            cfg: Arc::new(cfg),
            db: Arc::new(db),
            rpc: Arc::new(rpc),
        }))
    }
    pub async fn start(
        self: &Arc<Self>,
        mut exit_chan: tokio::sync::oneshot::Receiver<bool>,
    ) -> Result<()> {
        let mut ticker = tokio::time::interval(tokio::time::Duration::from_secs(
            self.cfg.liquidator.frequency,
        ));
        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    match self.db.list_obligations(Some(85.0)) {
                        Ok(obligations) => {
                            obligations.into_iter().for_each(|obligation| {
                                let service = self.clone();
                                let obligation = obligation.clone();
                                tokio::task::spawn(async move {
                                    match service.handle_liquidation_check(&obligation).await {
                                        Ok(_) => (),
                                        Err(err) => error!(
                                            "liquidation for obligation {} failed: {:#?}",
                                            obligation.account, err,
                                        ),
                                    }
                                });
                            });
                        },
                        Err(err) => {
                            log::error!("failed to list obligations {:#?}", err);
                        }
                    }
                }
                _ = &mut exit_chan => {
                    log::warn!("received exit channel");
                    return Ok(());
                }
            }
        }
    }
}
