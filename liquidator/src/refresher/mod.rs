//! service responsible for ensuring all obligations stored in the database
//! have their ltv values periodically refreshed
use std::sync::Arc;
pub mod refresh;
use anchor_lang::prelude::*;
use anyhow::{Result};
use chrono::Utc;
use config::Configuration;
use crossbeam::select;
use crossbeam_channel::tick;
use db::LiquidatorDb;
use diesel::r2d2;
use diesel::PgConnection;
use log::{error};
use rayon::ThreadPoolBuilder;

use solana_client::rpc_client::RpcClient;


use std::str::FromStr;

use crate::refresher::refresh::handle_pseudo_obligation_refresh;

pub struct Refresher {
    pub cfg: Arc<Configuration>,
    pub db: LiquidatorDb,
    pub rpc: Arc<RpcClient>,
}

impl Refresher {
    pub fn new(cfg: Configuration) -> Result<Arc<Refresher>> {
        let db = db::LiquidatorDb::new(cfg.clone())?;
        let rpc = cfg.get_rpc_client(false, None);
        Ok(Arc::new(Refresher {
            cfg: Arc::new(cfg),
            db,
            rpc: Arc::new(rpc),
        }))
    }
    pub async fn start(self: &Arc<Self>, _exit_chan: crossbeam_channel::Receiver<bool>) -> Result<()> {
        let ticker = tokio::time::interval(tokio::time::Duration::from_secs(self.cfg.refresher.frequency));
        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    match self.db.list_obligations(None) {
                        Ok(obligations) => {
                            use crate::awaitgroup::WaitGroup;

                            let mut wg = WaitGroup::new();

                            let (obligation_sender, obligation_receiver) = tokio::sync::mpsc::channel(obligations.len());


                            obligations.iter_mut().for_each(|obligation| {
                                let service = self.clone();
                                let wg = wg.worker();
                                let sender = obligation_sender.clone();
                                tokio::task::spawn(async {
                                    match handle_pseudo_obligation_refresh(
                                        &service.rpc,
                                        &obligation,
                                    ) {
                                        Ok(refreshed_obligation) => {
                                            match f64::from_str(&ltv.to_string()) {
                                                Ok(ltv) => {
                                                    obligation.ltv = ltv;
                                                    // finished processing, send update
                                                    if let Err(err) = sender.send(obligation.to_owned()).await {
                                                        log::error!("failed to send refreshed obligation {:#?}", err);
                                                    }
                                                },
                                                Err(err) => {
                                                    error!("failed to parse ltv to float {}: {:#?}", obligation.account, err);
                                                }
                                            }
                                        }
                                        Err(err) => {
                                            log::error!("failed to pseudo refresh obligation {}: {:#?}", obligation.account, err);
                                        }
                                    }
                                    wg.done();
                                });
                            });
                            // wait for all obligation refresh routines to finish
                            wg.wait().await;
                            
                            obligation_receiver.collect();
                        },
                        Err(err) => {
                            log::error!("failed to list obligations {:#?}", err);
                        }
                    }
                }
            }
        }
    }
}
