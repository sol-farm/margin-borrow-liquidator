use anyhow::Result;
use chrono::Utc;
use config::Configuration;
use crossbeam::{select, sync::WaitGroup};
use crossbeam_channel::tick;

use diesel::{
    r2d2::{ConnectionManager, Pool},
    PgConnection,
};
use log::{error, info, warn};
use solana_client::rpc_client::RpcClient;
use std::sync::Arc;

pub struct Service {
    pub config: Arc<Configuration>,
    pub rpc: Arc<RpcClient>,
    pub pool: Pool<ConnectionManager<PgConnection>>,
}

impl Service {
    pub fn new(config: Arc<Configuration>) -> Result<Arc<Self>> {
        let rpc = Arc::new(config.get_rpc_client(false, None));
        let pool = db::new_connection_pool(
            config.database.conn_url.clone(),
            config.database.analytics_pool_size,
        )?;
        Ok(Arc::new(Self { config, rpc, pool }))
    }
    pub fn start(self: &Arc<Self>, exit_chan: crossbeam_channel::Receiver<bool>) -> Result<()> {
        let price_feed_map = Arc::new(self.config.analytics.price_account_map());
        let reserve_account_map = Arc::new(self.config.analytics.reserve_map());
        let work_loop = || {
            let scraped_at = Utc::now();
            let wg = WaitGroup::new();
            // start the obligation account scraper
            {
                match self.pool.get() {
                    Ok(conn) => {
                        let wg = wg.clone();
                        let scraped_at = scraped_at;
                        let reserve_account_map = Arc::clone(&reserve_account_map);
                        let service = Arc::clone(self);
                        let reserve_account_map = match service.config.get_reserve_infos(
                            &service.rpc, 
                            &reserve_account_map
                        ) {
                            Ok(reserve_account_map) => reserve_account_map,
                            Err(err) => {
                                log::error!("failed to load reserve accounts {:#?}", err);
                                return;
                            }
                        };
                        tokio::task::spawn_blocking(move || {
                            info!("initiating obligation account scraper");
                            if let Err(err) = scrapers::lending_obligation::scrape_lending_obligations(
                                &service.config,
                                &service.rpc,
                                reserve_account_map,
                            ) {
                                log::error!("failed to scrape lending obligation {:#?}", err);
                            };
                            info!("finished obligation account scraping");
                            drop(wg);
                        });
                    }
                    Err(err) => {
                        error!("failed to get pool connection {:#?}", err);
                        return;
                    }
                }
            }
            // start the price feed scraper
            {
                match self.pool.get() {
                    Ok(conn) => {
                        let wg = wg.clone();
                        let scraped_at = scraped_at;
                        let service = Arc::clone(self);
                        let price_feed_map = Arc::clone(&price_feed_map);
                        tokio::task::spawn_blocking(move || {
                            info!("initiating price feed scraper");
                            scrapers::price_feeds::scrape_price_feeds(
                                &service.config,
                                &service.rpc,
                                &conn,
                                &price_feed_map,
                                scraped_at,
                            );
                            info!("finished price feed scraping");
                            drop(wg);
                        });
                    }
                    Err(err) => {
                        error!("failed to get pool connection {:#?}", err);
                        return;
                    }
                }
            }
            wg.wait();
        };
        info!("starting initial analytics run on startup");
        work_loop();
        let ticker = tick(std::time::Duration::from_secs(
            self.config.analytics.scrape_interval,
        ));
        loop {
            select! {
                recv(ticker) -> _msg => {
                    work_loop();
                },
                recv(exit_chan) -> msg => {
                    warn!("caught exit signal {:#?}", msg);
                    return Ok(());
                }
            }
        }
    }
}
