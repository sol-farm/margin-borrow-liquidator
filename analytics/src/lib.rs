use anyhow::Result;
use chrono::Utc;
use config::Configuration;
use crossbeam::{select, sync::WaitGroup};
use crossbeam_channel::tick;

use db::LiquidatorDb;
use diesel::{
    r2d2::{ConnectionManager, Pool},
    PgConnection,
};
use log::{error, info, warn};
use solana_client::nonblocking::rpc_client::RpcClient;
use std::sync::Arc;

pub struct Service {
    pub config: Arc<Configuration>,
    pub rpc: Arc<RpcClient>,
    pub db: Arc<LiquidatorDb>,
}

impl Service {
    pub fn new(config: Configuration) -> Result<Arc<Self>> {
        let rpc = Arc::new(config.get_rpc_client(false, None));
        let db = Arc::new(LiquidatorDb::new(config.clone())?);
        Ok(Arc::new(Self {
            config: Arc::new(config),
            rpc,
            db,
        }))
    }
    pub async fn start(
        self: &Arc<Self>,
        mut exit_chan: tokio::sync::oneshot::Receiver<bool>,
    ) -> Result<()> {
        let price_feed_map = Arc::new(self.config.analytics.price_account_map());
        let reserve_account_map = Arc::new(self.config.analytics.reserve_map());
        let ticker = tick(std::time::Duration::from_secs(
            self.config.analytics.scrape_interval,
        ));
        let mut ticker = tokio::time::interval(tokio::time::Duration::from_secs(
            self.config.analytics.scrape_interval,
        ));
        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    let scraped_at = Utc::now();
                    let wg = WaitGroup::new();
                    // start the obligation account scraper
                    {
                        let reserve_account_map = Arc::clone(&reserve_account_map);
                        let service = self.clone();
                        let reserve_account_map = match service
                            .config
                            .get_reserve_infos(&service.rpc, &reserve_account_map).await
                        {
                            Ok(reserve_account_map) => reserve_account_map,
                            Err(err) => {
                                log::error!("failed to load reserve accounts {:#?}", err);
                                continue;
                            }
                        };
                        let wg = wg.clone();
                        tokio::task::spawn(async move {
                            info!("initiating obligation account scraper");
                            if let Err(err) = scrapers::lending_obligation::scrape_lending_obligations(
                                &service.config,
                                &service.rpc,
                                reserve_account_map,
                            ).await {
                                log::error!("failed to scrape lending obligation {:#?}", err);
                            };

                            info!("finished obligation account scraping");
                            drop(wg);
                        });
                    }
                    // start the price feed scraper
                    {
                        let wg = wg.clone();
                        let scraped_at = scraped_at;
                        let service = Arc::clone(self);
                        let price_feed_map = price_feed_map.clone();
                        tokio::task::spawn(async move {
                            info!("initiating price feed scraper");
                            scrapers::price_feeds::scrape_price_feeds(
                                &service.config,
                                &service.rpc,
                                &service.db,
                                &price_feed_map,
                                scraped_at,
                            ).await;
                            info!("finished price feed scraping");
                            drop(wg);
                        });
                    }
                    wg.wait();
                }
                _ = &mut exit_chan => {
                    log::warn!("received exit signal");
                    return Ok(());
                }
            }
        }
    }
}
