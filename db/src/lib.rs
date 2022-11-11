#![allow(clippy::needless_lifetimes)]
#![allow(clippy::bool_assert_comparison)]
#![allow(clippy::too_many_arguments)]
#![deny(unused_must_use)]

pub mod models;
pub mod utils;
use crate::models::PriceFeed;
use anyhow::{anyhow, Result};
use bonerjams_db::types::DbTrees;
use bonerjams_db::DbBatch;
use config::Configuration;
use models::Obligation;
use solana_sdk::pubkey::Pubkey;
use std::sync::Arc;

/// the key used by the tree storing obligation ltv information
pub const OBLIGATION_TREE_KEY: &[u8] = b"OBLIGATIONS";
/// the key used by the tree storing price feed information
pub const PRICE_FEED_TREE_KEY: &[u8] = b"PRICE_FEEDS";

#[derive(Clone)]
pub struct LiquidatorDb {
    db: Arc<bonerjams_db::Database>,
}

impl LiquidatorDb {
    pub fn new(cfg: Configuration) -> Result<LiquidatorDb> {
        let bdb = bonerjams_db::Database::new(&cfg.sled_db.db)?;
        Ok(Self { db: bdb })
    }
    pub fn delete_obligations(&self, account: &[Pubkey]) -> Result<()> {
        let db_tree = self.db.open_tree(DbTrees::Binary(OBLIGATION_TREE_KEY))?;
        let mut batch = DbBatch::new();
        if let Err(err) = account.iter().try_for_each(|account| {
            if let Err(err) = batch.remove_raw(&account.to_bytes()[..]) {
                return Err(anyhow!("batch insertion failed {:#?}", err));
            }
            Ok(())
        }) {
            log::error!("{}", err);
        }
        db_tree.apply_batch(&mut batch)?;
        Ok(())
    }
    /// lists all known obligations, pre-sorting the vector ordering by ltv from lowest -> greater
    /// with any obligations containin NaN values filtered out.
    ///
    /// if `min_ltv` is Some(val), any obligations with an ltv below `val` are filtered out
    pub fn list_obligations(&self, min_ltv: Option<f64>) -> Result<Vec<Obligation>> {
        use itertools::Itertools;
        let db_tree = self.db.open_tree(DbTrees::Binary(OBLIGATION_TREE_KEY))?;
        let iter = db_tree
            .iter()
            .flatten()
            .filter_map(|(_, value)| {
                let obligation: Obligation = match serde_json::from_slice(&value) {
                    Ok(obligation) => obligation,
                    Err(err) => {
                        log::error!("failed to deserialize obligation {:#?}", err);
                        return None;
                    }
                };
                if obligation.ltv.is_nan() {
                    return None;
                }
                Some(obligation)
            })
            .sorted_unstable_by(crate::utils::cmp_ltvs)
            .collect::<Vec<_>>();
        if let Some(min_ltv) = min_ltv {
            Ok(iter
                .into_iter()
                .filter(|obligation| obligation.ltv.ge(&min_ltv))
                .collect::<Vec<_>>())
        } else {
            Ok(iter)
        }
    }
    pub fn list_price_feeds(&self) -> Result<Vec<PriceFeed>> {
        let db_tree = self.db.open_tree(DbTrees::Binary(PRICE_FEED_TREE_KEY))?;
        Ok(db_tree
            .iter()
            .flatten()
            .filter_map(|(_, value)| {
                let price_feed: PriceFeed = match serde_json::from_slice(&value) {
                    Ok(price_feed) => price_feed,
                    Err(err) => {
                        log::error!("failed to deserialize price_feed {:#?}", err);
                        return None;
                    }
                };
                Some(price_feed)
            })
            .collect::<Vec<_>>())
    }
    pub fn insert_price_feeds(&self, price_feeds: &[PriceFeed]) -> Result<()> {
        let db_tree = self.db.open_tree(DbTrees::Binary(PRICE_FEED_TREE_KEY))?;
        let mut batch = DbBatch::new();
        if let Err(err) = price_feeds.iter().try_for_each(|price_feed| {
            if let Err(err) = batch.insert(price_feed) {
                return Err(anyhow!("batch insertion failed {:#?}", err));
            }
            Ok(())
        }) {
            return Err(anyhow!("{}", err));
        }
        db_tree.apply_batch(&mut batch)?;
        Ok(())
    }
    pub fn insert_obligations(&self, obligations: &[Obligation]) -> Result<()> {
        let db_tree = self.db.open_tree(DbTrees::Binary(OBLIGATION_TREE_KEY))?;
        let mut batch = DbBatch::new();
        if let Err(err) = obligations.iter().try_for_each(|obligation| {
            if let Err(err) = batch.insert(obligation) {
                return Err(anyhow!("batch insertion failed {:#?}", err));
            }
            Ok(())
        }) {
            return Err(anyhow!("{}", err));
        }
        db_tree.apply_batch(&mut batch)?;
        Ok(())
    }
    pub fn db(&self) -> Arc<bonerjams_db::Database> {
        self.db.clone()
    }
}

#[cfg(test)]
mod test {
    use solana_sdk::pubkey::Pubkey;

    use super::*;
    #[test]
    fn test_database() {
        {
            let mut cfg = Configuration::default();
            cfg.sled_db.db.path = "test_database_tmp.db".to_string();

            let db = LiquidatorDb::new(cfg).unwrap();

            const NUM_FEEDS: usize = 5;
            const NUM_OBLIGATIONS: usize = 1000;

            let mut price_feeds = Vec::with_capacity(5);
            for _ in 0..NUM_FEEDS {
                price_feeds.push(new_price_feed());
            }

            db.insert_price_feeds(&price_feeds[..]).unwrap();

            let got_price_feeds = db.list_price_feeds().unwrap();

            assert_eq!(got_price_feeds.len(), NUM_FEEDS);

            let mut obligations = Vec::with_capacity(NUM_OBLIGATIONS);
            for idx in 0..NUM_OBLIGATIONS {
                obligations.push(new_obligation(idx < 500));
            }

            db.insert_obligations(&obligations[..]).unwrap();

            let got_obligations = db.list_obligations(None).unwrap();

            assert_eq!(got_obligations.len(), NUM_OBLIGATIONS);
            // test that the obligations are sorted
            assert!(got_obligations[0].ltv < got_obligations[got_obligations.len() - 1].ltv);

            let got_obligations = db.list_obligations(Some(85.1)).unwrap();
            assert!(got_obligations.len() == 500);
            assert!(got_obligations[0].ltv < got_obligations[got_obligations.len() - 1].ltv);

            let got_obligations = db.list_obligations(Some(95.0)).unwrap();
            assert!(!got_obligations.is_empty() && got_obligations.len() <= 500);
            assert!(got_obligations[0].ltv < got_obligations[got_obligations.len() - 1].ltv);

            let obligations_to_delete = got_obligations
                .iter()
                .map(|obligation| obligation.account)
                .collect::<Vec<_>>();
            let count = obligations_to_delete.len() / 2;
            let obligations_to_delete = obligations_to_delete
                .into_iter()
                .take(count)
                .collect::<Vec<_>>();
            println!("deleting {}", obligations_to_delete.len());
            db.delete_obligations(&obligations_to_delete[..]).unwrap();

            let got_obligations = db.list_obligations(None).unwrap();
            assert!(!got_obligations.is_empty() && got_obligations.len() < NUM_OBLIGATIONS);
        }
        std::fs::remove_dir_all("test_database_tmp.db").unwrap();
    }

    pub fn new_price_feed() -> PriceFeed {
        use chrono::prelude::*;
        use rand::prelude::*;
        let mut rng = rand::thread_rng();
        PriceFeed {
            token_mint: Pubkey::new_unique(),
            price_account: Pubkey::new_unique(),
            decimals: 6,
            price: rng.gen(),
            scraped_at: Utc::now(),
        }
    }
    /// if underwater is true, then we generate a random
    /// obligation with an ltv above 85%
    pub fn new_obligation(underwater: bool) -> Obligation {
        use chrono::prelude::*;
        use rand::prelude::*;
        let mut rng = rand::thread_rng();
        let ltv: f64 = if underwater {
            rng.gen_range(85.1, 100.0)
        } else {
            rng.gen_range(0.0, 84.9)
        };
        Obligation {
            account: Pubkey::new_unique(),
            ltv,
            scraped_at: Utc::now(),
        }
    }
}
