use crate::filters::{
    FindObligation, FindPriceFeed, PriceFeedMatcher, ObligationMatcher,
};
use crate::models::{
    Obligation, PriceFeed
};
use crate::schema::*;
use ::r2d2::Pool;
use anyhow::{anyhow, Result};
use arrform::{arrform, ArrForm};
use chrono::{prelude::*, Duration};
use diesel::r2d2;
use diesel::r2d2::ConnectionManager;
use diesel::PgConnection;
use diesel::*;
use diesel_derives_traits::{Model, NewModel};
use into_query::IntoQuery;
use log::{info, warn, error};
use std::convert::TryInto;
use std::sync::Arc;


#[derive(Debug, Insertable, NewModel)]
#[table_name = "obligations"]
#[model(Obligation)]
pub struct NewObligation {
    pub ltv: f64,
    pub account: String,
    pub account_data: Option<Vec<u8>>,
    pub scraped_at: DateTime<Utc>,
}


#[derive(Debug, Insertable, NewModel)]
#[table_name = "price_feeds"]
#[model(PriceFeed)]
pub struct NewPriceFeed {
    pub token_mint: String,
    pub price_account: String,
    pub decimals: i16,
    pub price: f64,
    pub scraped_at: DateTime<Utc>,
}

/// puts a price feed update into the database, updating
/// the record if it already exists, creating a new one if it does not
pub fn put_price_feed(
    conn: &PgConnection,
    token_mint: &str,
    price_account: &str,
    decimals: i16,
    price: f64,
    scraped_at: DateTime<Utc>,
) -> Result<()> {
    conn.transaction(|| {
        let mut results = get_price_feed(
            conn,
            &PriceFeedMatcher::PriceAccount(vec![price_account.to_string()])
        )?;
        if results.is_empty() {
            // no record, create it
            NewPriceFeed {
                token_mint: token_mint.to_string(),
                price_account: price_account.to_string(),
                decimals,
                price,
                scraped_at,
            }.save(conn)?;
        } else {
            let mut price_feed = std::mem::take(&mut results[0]);
            price_feed.price = price;
            price_feed.scraped_at = scraped_at;
            price_feed.save(conn)?;
        }
        Ok(())
    })
}


/// returns any price feeds matched by the given matcher 
pub fn get_price_feed(
    conn: &PgConnection,
    matcher: &PriceFeedMatcher,   
) -> QueryResult<Vec<PriceFeed>> {
    matcher
    .to_filter()
    .into_query()
    .get_results::<PriceFeed>(conn)
}

/// deletes any price feeds matched by the given matcher
pub fn delete_price_feed(
    conn: &PgConnection,
    matcher: &PriceFeedMatcher,     
) -> Result<()> {
    let mut results = get_price_feed(conn, matcher)?;
    for result in &mut results {
        std::mem::take(result).destroy(conn)?;
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    #[allow(unused_must_use)]
    fn test_price_feed() {
        use crate::test_utils::TestDb;
        std::env::set_var(
            "DATABASE_URL",
            "postgres://postgres:password123@localhost/liquidator",
        );
        let test_db = TestDb::new();
        let conn = test_db.conn();
        crate::run_migrations(&conn);

        let scraped_at_one = Utc::now();

        // give time for database to do its thing
        std::thread::sleep(std::time::Duration::from_secs(2));
        let cleanup = || {
            delete_price_feed(&conn, &PriceFeedMatcher::All);
        };
        cleanup();
        // give time for database to do its thing
        std::thread::sleep(std::time::Duration::from_secs(2));

        let scraped_at_two = Utc::now();

        let token_mint_1 = "mint-1";
        let price_account_1 = "price-1";
        let decimals_1 = 6_i16;
        let price_1 = 420.69;

        let token_mint_2 = "mint-2";
        let price_account_2 = "price-2";
        let decimals_2 = 9_i16;
        let price_2 = 69.420;

        // we should have no results 
        let results = get_price_feed(&conn, &PriceFeedMatcher::All).unwrap();
        assert_eq!(results.len(), 0);
        let results = get_price_feed(&conn, &PriceFeedMatcher::PriceAccount(vec![price_account_1.to_string()])).unwrap();
        assert_eq!(results.len(), 0);
        let results = get_price_feed(&conn, &PriceFeedMatcher::TokenMint(vec![token_mint_1.to_string()])).unwrap();
        assert_eq!(results.len(), 0);

        // test the first update to a price feed, creating it
        {
            put_price_feed(
                &conn,
                token_mint_1,
                price_account_1,
                decimals_1,
                price_1,
                scraped_at_one,
            ).unwrap();

            let results = get_price_feed(&conn, &PriceFeedMatcher::All).unwrap();
            assert_eq!(results.len(), 1);
            assert_eq!(results[0].token_mint, token_mint_1.to_string());
            assert_eq!(results[0].price_account, price_account_1.to_string());
            assert_eq!(results[0].decimals, decimals_1);
            assert_eq!(results[0].scraped_at.hour(), scraped_at_one.hour());
            assert_eq!(results[0].scraped_at.minute(), scraped_at_one.minute());
            assert_eq!(results[0].scraped_at.second(), scraped_at_one.second());
            assert_eq!(results[0].price, price_1);
            let results = get_price_feed(&conn, &PriceFeedMatcher::PriceAccount(vec![price_account_1.to_string()])).unwrap();
            assert_eq!(results.len(), 1);
            assert_eq!(results[0].token_mint, token_mint_1.to_string());
            assert_eq!(results[0].price_account, price_account_1.to_string());
            assert_eq!(results[0].decimals, decimals_1);
            assert_eq!(results[0].scraped_at.hour(), scraped_at_one.hour());
            assert_eq!(results[0].scraped_at.minute(), scraped_at_one.minute());
            assert_eq!(results[0].scraped_at.second(), scraped_at_one.second());
            assert_eq!(results[0].price, price_1);
            let results = get_price_feed(&conn, &PriceFeedMatcher::TokenMint(vec![token_mint_1.to_string()])).unwrap();
            assert_eq!(results.len(), 1);
            assert_eq!(results[0].token_mint, token_mint_1.to_string());
            assert_eq!(results[0].price_account, price_account_1.to_string());
            assert_eq!(results[0].decimals, decimals_1);
            assert_eq!(results[0].scraped_at.hour(), scraped_at_one.hour());
            assert_eq!(results[0].scraped_at.minute(), scraped_at_one.minute());
            assert_eq!(results[0].scraped_at.second(), scraped_at_one.second());
            assert_eq!(results[0].price, price_1);
        }
        // test second update to a price feed, updating it
        {
            put_price_feed(
                &conn,
                token_mint_1,
                price_account_1,
                decimals_1,
                69.69,
                scraped_at_one,
            ).unwrap();
            
            let results = get_price_feed(&conn, &PriceFeedMatcher::All).unwrap();
            assert_eq!(results.len(), 1);
            assert_eq!(results[0].token_mint, token_mint_1.to_string());
            assert_eq!(results[0].price_account, price_account_1.to_string());
            assert_eq!(results[0].decimals, decimals_1);
            assert_eq!(results[0].scraped_at.hour(), scraped_at_one.hour());
            assert_eq!(results[0].scraped_at.minute(), scraped_at_one.minute());
            assert_eq!(results[0].scraped_at.second(), scraped_at_one.second());
            assert_eq!(results[0].price, 69.69);
            let results = get_price_feed(&conn, &PriceFeedMatcher::PriceAccount(vec![price_account_1.to_string()])).unwrap();
            assert_eq!(results.len(), 1);
            assert_eq!(results[0].token_mint, token_mint_1.to_string());
            assert_eq!(results[0].price_account, price_account_1.to_string());
            assert_eq!(results[0].decimals, decimals_1);
            assert_eq!(results[0].scraped_at.hour(), scraped_at_one.hour());
            assert_eq!(results[0].scraped_at.minute(), scraped_at_one.minute());
            assert_eq!(results[0].scraped_at.second(), scraped_at_one.second());
            assert_eq!(results[0].price, 69.69);
            let results = get_price_feed(&conn, &PriceFeedMatcher::TokenMint(vec![token_mint_1.to_string()])).unwrap();
            assert_eq!(results.len(), 1);
            assert_eq!(results[0].token_mint, token_mint_1.to_string());
            assert_eq!(results[0].price_account, price_account_1.to_string());
            assert_eq!(results[0].decimals, decimals_1);
            assert_eq!(results[0].scraped_at.hour(), scraped_at_one.hour());
            assert_eq!(results[0].scraped_at.minute(), scraped_at_one.minute());
            assert_eq!(results[0].scraped_at.second(), scraped_at_one.second());
            assert_eq!(results[0].price, 69.69);
        }
        // test again but with a different price feed
        {
            put_price_feed(
                &conn,
                token_mint_2,
                price_account_2,
                decimals_2,
                price_2,
                scraped_at_two,
            ).unwrap();

            let results = get_price_feed(&conn, &PriceFeedMatcher::All).unwrap();
            assert_eq!(results.len(), 2);
            let results = get_price_feed(&conn, &PriceFeedMatcher::PriceAccount(vec![price_account_2.to_string()])).unwrap();
            assert_eq!(results.len(), 1);
            assert_eq!(results[0].token_mint, token_mint_2.to_string());
            assert_eq!(results[0].price_account, price_account_2.to_string());
            assert_eq!(results[0].decimals, decimals_2);
            assert_eq!(results[0].scraped_at.hour(), scraped_at_two.hour());
            assert_eq!(results[0].scraped_at.minute(), scraped_at_two.minute());
            assert_eq!(results[0].scraped_at.second(), scraped_at_two.second());
            assert_eq!(results[0].price, price_2);
            let results = get_price_feed(&conn, &PriceFeedMatcher::TokenMint(vec![token_mint_2.to_string()])).unwrap();
            assert_eq!(results.len(), 1);
            assert_eq!(results[0].token_mint, token_mint_2.to_string());
            assert_eq!(results[0].price_account, price_account_2.to_string());
            assert_eq!(results[0].decimals, decimals_2);
            assert_eq!(results[0].scraped_at.hour(), scraped_at_two.hour());
            assert_eq!(results[0].scraped_at.minute(), scraped_at_two.minute());
            assert_eq!(results[0].scraped_at.second(), scraped_at_two.second());
            assert_eq!(results[0].price, price_2);
        }
        // test again but with a different price feed
        {
            put_price_feed(
                &conn,
                token_mint_2,
                price_account_2,
                decimals_2,
                420.42069,
                scraped_at_two,
            ).unwrap();

            let results = get_price_feed(&conn, &PriceFeedMatcher::All).unwrap();
            assert_eq!(results.len(), 2);
            let results = get_price_feed(&conn, &PriceFeedMatcher::PriceAccount(vec![price_account_2.to_string()])).unwrap();
            assert_eq!(results.len(), 1);
            assert_eq!(results[0].token_mint, token_mint_2.to_string());
            assert_eq!(results[0].price_account, price_account_2.to_string());
            assert_eq!(results[0].decimals, decimals_2);
            assert_eq!(results[0].scraped_at.hour(), scraped_at_two.hour());
            assert_eq!(results[0].scraped_at.minute(), scraped_at_two.minute());
            assert_eq!(results[0].scraped_at.second(), scraped_at_two.second());
            assert_eq!(results[0].price, 420.42069);
            let results = get_price_feed(&conn, &PriceFeedMatcher::TokenMint(vec![token_mint_2.to_string()])).unwrap();
            assert_eq!(results.len(), 1);
            assert_eq!(results[0].token_mint, token_mint_2.to_string());
            assert_eq!(results[0].price_account, price_account_2.to_string());
            assert_eq!(results[0].decimals, decimals_2);
            assert_eq!(results[0].scraped_at.hour(), scraped_at_two.hour());
            assert_eq!(results[0].scraped_at.minute(), scraped_at_two.minute());
            assert_eq!(results[0].scraped_at.second(), scraped_at_two.second());
            assert_eq!(results[0].price, 420.42069);
        }
        cleanup();
    }
}