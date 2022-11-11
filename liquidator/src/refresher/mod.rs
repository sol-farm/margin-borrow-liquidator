//! service responsible for ensuring all obligations stored in the database
//! have their ltv values periodically refreshed
use std::sync::Arc;
pub mod refresh;
use anyhow::Result;
use bonerjams_db::DbBatch;
use chrono::Utc;
use config::Configuration;
use crossbeam::select;
use crossbeam_channel::tick;
use db::LiquidatorDb;
use diesel::r2d2;
use diesel::PgConnection;
use futures_util::FutureExt;
use log::error;
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
    pub async fn start(
        self: &Arc<Self>,
        mut exit_chan: tokio::sync::oneshot::Receiver<bool>,
    ) -> Result<()> {
        let mut ticker = tokio::time::interval(tokio::time::Duration::from_secs(
            self.cfg.refresher.frequency,
        ));
        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    match self.db.list_obligations(None) {
                        Ok(obligations) => {
                            use crate::awaitgroup::WaitGroup;
                            let obligations_count = obligations.len();
                            let mut wg = WaitGroup::new();

                            let (obligation_sender, mut obligation_receiver) = tokio::sync::mpsc::channel(obligations_count);


                            obligations.iter().for_each(|obligation| {
                                let mut obligation = obligation.clone();
                                let service = self.clone();
                                let wg = wg.worker();
                                let sender = obligation_sender.clone();
                                tokio::task::spawn(async move {
                                    match handle_pseudo_obligation_refresh(
                                        &service.rpc,
                                        &obligation,
                                    ) {
                                        Ok(refreshed_obligation) => {
                                            match refreshed_obligation.loan_to_value() {
                                                Ok(ltv) => {
                                                    match f64::from_str(&ltv.to_string()) {
                                                        Ok(ltv) => {
                                                            obligation.ltv = ltv;
                                                            // finished processing, send update
                                                            if sender.send(obligation.to_owned()).await.is_err() {
                                                                log::error!("failed to send refreshed obligation");
                                                            }
                                                        },
                                                        Err(err) => {
                                                            error!("failed to parse ltv to float {}: {:#?}", obligation.account, err);
                                                        }
                                                    }
                                                }
                                                Err(err) => {
                                                    log::error!("failed to parse ltv {}: {:#?}", obligation.account, err)
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
                            log::info!("waiting for routines to finish");
                            wg.wait().await;
                            let mut updates = Vec::with_capacity(obligations_count);
                            log::info!("processing obligation receiver");
                            while let Some(update) = obligation_receiver.recv().await {
                                updates.push(update);
                            }
                            log::info!("updating database");
                            if let Err(err) = self.db.insert_obligations(&updates[..]) {
                                log::error!("failed to insert obligations {:#?}", err);
                            }
                        },
                        Err(err) => {
                            log::error!("failed to list obligations {:#?}", err);
                        }
                    }
                }
                _ = &mut exit_chan => {
                    log::warn!("receive exit signal");
                    return Ok(());
                }
            }
        }
    }
}
