use crate::filters::{LtvFilter, ObligationMatcher, PriceFeedMatcher};
use crate::models::{Obligation, PriceFeed};
use crate::schema::*;
use anyhow::Result;

use chrono::prelude::*;

use diesel::PgConnection;
use diesel::*;
use diesel_derives_traits::{Model, NewModel};
use into_query::IntoQuery;

#[derive(Debug, Insertable, NewModel)]
#[table_name = "obligations"]
#[model(Obligation)]
pub struct NewObligation {
    pub ltv: f64,
    pub account: String,
    pub account_data: Vec<u8>,
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
            &PriceFeedMatcher::PriceAccount(vec![price_account.to_string()]),
        )?;
        if results.is_empty() {
            // no record, create it
            NewPriceFeed {
                token_mint: token_mint.to_string(),
                price_account: price_account.to_string(),
                decimals,
                price,
                scraped_at,
            }
            .save(conn)?;
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
pub fn delete_price_feed(conn: &PgConnection, matcher: &PriceFeedMatcher) -> Result<()> {
    let mut results = get_price_feed(conn, matcher)?;
    for result in &mut results {
        std::mem::take(result).destroy(conn)?;
    }
    Ok(())
}

/// puts an obligation account update into the database, updating
/// the record if it already exists, creating a new one if it does not
pub fn put_obligation(
    conn: &PgConnection,
    ltv: f64,
    account: &str,
    account_data: &[u8],
    scraped_at: DateTime<Utc>,
) -> Result<()> {
    conn.transaction(|| {
        let mut results = get_obligation(
            conn,
            &ObligationMatcher::Account(vec![account.to_string()]),
            None,
        )?;
        if results.is_empty() {
            // no record, create it
            NewObligation {
                ltv,
                account: account.to_string(),
                account_data: account_data.to_vec(),
                scraped_at,
            }
            .save(conn)?;
        } else {
            let mut obligation = std::mem::take(&mut results[0]);
            obligation.account_data = account_data.to_vec();
            obligation.ltv = ltv;
            obligation.save(conn)?;
        }
        Ok(())
    })
}

/// returns any obligations matched by the given matcher
pub fn get_obligation(
    conn: &PgConnection,
    matcher: &ObligationMatcher,
    ltv_filter: Option<LtvFilter>,
) -> QueryResult<Vec<Obligation>> {
    use crate::schema::obligations::dsl::*;
    let query = matcher.to_filter().into_query();
    let query = if let Some(ltv_filter) = ltv_filter {
        match ltv_filter {
            LtvFilter::GE(ltv_val) => query.filter(ltv.ge(ltv_val)),
            LtvFilter::LE(ltv_val) => query.filter(ltv.le(ltv_val)),
            LtvFilter::GT(ltv_val) => query.filter(ltv.gt(ltv_val)),
            LtvFilter::LT(ltv_val) => query.filter(ltv.lt(ltv_val)),
        }
    } else {
        query
    };
    query.get_results::<Obligation>(conn)
}

/// deletes any price feeds matched by the given matcher
pub fn delete_obligation(
    conn: &PgConnection,
    matcher: &ObligationMatcher,
    ltv_filter: Option<LtvFilter>,
) -> Result<()> {
    let mut results = get_obligation(conn, matcher, ltv_filter)?;
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
        let results = get_price_feed(
            &conn,
            &PriceFeedMatcher::PriceAccount(vec![price_account_1.to_string()]),
        )
        .unwrap();
        assert_eq!(results.len(), 0);
        let results = get_price_feed(
            &conn,
            &PriceFeedMatcher::TokenMint(vec![token_mint_1.to_string()]),
        )
        .unwrap();
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
            )
            .unwrap();

            let results = get_price_feed(&conn, &PriceFeedMatcher::All).unwrap();
            assert_eq!(results.len(), 1);
            assert_eq!(results[0].token_mint, token_mint_1.to_string());
            assert_eq!(results[0].price_account, price_account_1.to_string());
            assert_eq!(results[0].decimals, decimals_1);
            assert_eq!(results[0].scraped_at.hour(), scraped_at_one.hour());
            assert_eq!(results[0].scraped_at.minute(), scraped_at_one.minute());
            assert_eq!(results[0].scraped_at.second(), scraped_at_one.second());
            assert_eq!(results[0].price, price_1);
            let results = get_price_feed(
                &conn,
                &PriceFeedMatcher::PriceAccount(vec![price_account_1.to_string()]),
            )
            .unwrap();
            assert_eq!(results.len(), 1);
            assert_eq!(results[0].token_mint, token_mint_1.to_string());
            assert_eq!(results[0].price_account, price_account_1.to_string());
            assert_eq!(results[0].decimals, decimals_1);
            assert_eq!(results[0].scraped_at.hour(), scraped_at_one.hour());
            assert_eq!(results[0].scraped_at.minute(), scraped_at_one.minute());
            assert_eq!(results[0].scraped_at.second(), scraped_at_one.second());
            assert_eq!(results[0].price, price_1);
            let results = get_price_feed(
                &conn,
                &PriceFeedMatcher::TokenMint(vec![token_mint_1.to_string()]),
            )
            .unwrap();
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
            )
            .unwrap();

            let results = get_price_feed(&conn, &PriceFeedMatcher::All).unwrap();
            assert_eq!(results.len(), 1);
            assert_eq!(results[0].token_mint, token_mint_1.to_string());
            assert_eq!(results[0].price_account, price_account_1.to_string());
            assert_eq!(results[0].decimals, decimals_1);
            assert_eq!(results[0].scraped_at.hour(), scraped_at_one.hour());
            assert_eq!(results[0].scraped_at.minute(), scraped_at_one.minute());
            assert_eq!(results[0].scraped_at.second(), scraped_at_one.second());
            assert_eq!(results[0].price, 69.69);
            let results = get_price_feed(
                &conn,
                &PriceFeedMatcher::PriceAccount(vec![price_account_1.to_string()]),
            )
            .unwrap();
            assert_eq!(results.len(), 1);
            assert_eq!(results[0].token_mint, token_mint_1.to_string());
            assert_eq!(results[0].price_account, price_account_1.to_string());
            assert_eq!(results[0].decimals, decimals_1);
            assert_eq!(results[0].scraped_at.hour(), scraped_at_one.hour());
            assert_eq!(results[0].scraped_at.minute(), scraped_at_one.minute());
            assert_eq!(results[0].scraped_at.second(), scraped_at_one.second());
            assert_eq!(results[0].price, 69.69);
            let results = get_price_feed(
                &conn,
                &PriceFeedMatcher::TokenMint(vec![token_mint_1.to_string()]),
            )
            .unwrap();
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
            )
            .unwrap();

            let results = get_price_feed(&conn, &PriceFeedMatcher::All).unwrap();
            assert_eq!(results.len(), 2);
            let results = get_price_feed(
                &conn,
                &PriceFeedMatcher::PriceAccount(vec![price_account_2.to_string()]),
            )
            .unwrap();
            assert_eq!(results.len(), 1);
            assert_eq!(results[0].token_mint, token_mint_2.to_string());
            assert_eq!(results[0].price_account, price_account_2.to_string());
            assert_eq!(results[0].decimals, decimals_2);
            assert_eq!(results[0].scraped_at.hour(), scraped_at_two.hour());
            assert_eq!(results[0].scraped_at.minute(), scraped_at_two.minute());
            assert_eq!(results[0].scraped_at.second(), scraped_at_two.second());
            assert_eq!(results[0].price, price_2);
            let results = get_price_feed(
                &conn,
                &PriceFeedMatcher::TokenMint(vec![token_mint_2.to_string()]),
            )
            .unwrap();
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
            )
            .unwrap();

            let results = get_price_feed(&conn, &PriceFeedMatcher::All).unwrap();
            assert_eq!(results.len(), 2);
            let results = get_price_feed(
                &conn,
                &PriceFeedMatcher::PriceAccount(vec![price_account_2.to_string()]),
            )
            .unwrap();
            assert_eq!(results.len(), 1);
            assert_eq!(results[0].token_mint, token_mint_2.to_string());
            assert_eq!(results[0].price_account, price_account_2.to_string());
            assert_eq!(results[0].decimals, decimals_2);
            assert_eq!(results[0].scraped_at.hour(), scraped_at_two.hour());
            assert_eq!(results[0].scraped_at.minute(), scraped_at_two.minute());
            assert_eq!(results[0].scraped_at.second(), scraped_at_two.second());
            assert_eq!(results[0].price, 420.42069);
            let results = get_price_feed(
                &conn,
                &PriceFeedMatcher::TokenMint(vec![token_mint_2.to_string()]),
            )
            .unwrap();
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
    #[test]
    #[allow(unused_must_use)]
    fn test_obligation() {
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
            delete_obligation(&conn, &ObligationMatcher::All, None);
        };
        cleanup();
        // give time for database to do its thing
        std::thread::sleep(std::time::Duration::from_secs(2));

        let scraped_at_two = Utc::now();

        let ltv_one = 420.69;
        let account_one = "account-1";
        let account_data_one = "420".as_bytes().to_vec();

        let ltv_two = 69.420;
        let account_two = "account-2";
        let account_data_two = "69".as_bytes().to_vec();

        // we should have no results
        let results = get_obligation(&conn, &ObligationMatcher::All, None).unwrap();
        assert_eq!(results.len(), 0);
        let results = get_obligation(
            &conn,
            &ObligationMatcher::Account(vec![account_one.to_string()]),
            None,
        )
        .unwrap();
        assert_eq!(results.len(), 0);

        // test first update to an account, creating the record
        {
            put_obligation(
                &conn,
                ltv_one,
                account_one,
                &account_data_one[..],
                scraped_at_one,
            )
            .unwrap();

            let results = get_obligation(&conn, &ObligationMatcher::All, None).unwrap();
            assert_eq!(results.len(), 1);
            let results = get_obligation(
                &conn,
                &ObligationMatcher::Account(vec![account_one.to_string()]),
                None,
            )
            .unwrap();
            assert_eq!(results.len(), 1);
            assert_eq!(results[0].ltv, ltv_one);
            assert_eq!(results[0].account.to_string(), account_one.to_string());
            assert_eq!(results[0].account_data, account_data_one);
            assert_eq!(results[0].scraped_at.hour(), scraped_at_one.hour());
            assert_eq!(results[0].scraped_at.minute(), scraped_at_one.minute());
            assert_eq!(results[0].scraped_at.second(), scraped_at_one.second());
        }
        // test second update to an account, updating the record
        {
            let new_account_data = "696969".as_bytes().to_vec();
            put_obligation(
                &conn,
                ltv_one,
                account_one,
                &new_account_data[..],
                scraped_at_one,
            )
            .unwrap();

            let results = get_obligation(&conn, &ObligationMatcher::All, None).unwrap();
            assert_eq!(results.len(), 1);
            let results = get_obligation(
                &conn,
                &ObligationMatcher::Account(vec![account_one.to_string()]),
                None,
            )
            .unwrap();
            assert_eq!(results.len(), 1);
            assert_eq!(results[0].ltv, ltv_one);
            assert_eq!(results[0].account.to_string(), account_one.to_string());
            assert_eq!(results[0].account_data, new_account_data);
            assert_eq!(results[0].scraped_at.hour(), scraped_at_one.hour());
            assert_eq!(results[0].scraped_at.minute(), scraped_at_one.minute());
            assert_eq!(results[0].scraped_at.second(), scraped_at_one.second());
        }
        // test a new obligation
        {
            put_obligation(
                &conn,
                ltv_two,
                account_two,
                &account_data_two[..],
                scraped_at_two,
            )
            .unwrap();

            let results = get_obligation(&conn, &ObligationMatcher::All, None).unwrap();
            assert_eq!(results.len(), 2);
            let results = get_obligation(
                &conn,
                &ObligationMatcher::Account(vec![account_two.to_string()]),
                None,
            )
            .unwrap();
            assert_eq!(results.len(), 1);
            assert_eq!(results[0].ltv, ltv_two);
            assert_eq!(results[0].account.to_string(), account_two.to_string());
            assert_eq!(results[0].account_data, account_data_two);
            assert_eq!(results[0].scraped_at.hour(), scraped_at_two.hour());
            assert_eq!(results[0].scraped_at.minute(), scraped_at_two.minute());
            assert_eq!(results[0].scraped_at.second(), scraped_at_two.second());
        }
        // test updating a new obligation
        {
            let new_account_data_two = "496929123123".as_bytes().to_vec();
            put_obligation(
                &conn,
                ltv_two,
                account_two,
                &new_account_data_two[..],
                scraped_at_two,
            )
            .unwrap();

            let results = get_obligation(&conn, &ObligationMatcher::All, None).unwrap();
            assert_eq!(results.len(), 2);
            let results = get_obligation(
                &conn,
                &ObligationMatcher::Account(vec![account_two.to_string()]),
                None,
            )
            .unwrap();
            assert_eq!(results.len(), 1);
            assert_eq!(results[0].ltv, ltv_two);
            assert_eq!(results[0].account.to_string(), account_two.to_string());
            assert_eq!(results[0].account_data, new_account_data_two);
            assert_eq!(results[0].scraped_at.hour(), scraped_at_two.hour());
            assert_eq!(results[0].scraped_at.minute(), scraped_at_two.minute());
            assert_eq!(results[0].scraped_at.second(), scraped_at_two.second());
        }
    }
    #[test]
    #[allow(unused_must_use)]
    fn test_obligation_ltv_filter() {
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
            delete_obligation(&conn, &ObligationMatcher::All, None);
        };
        cleanup();
        // give time for database to do its thing
        std::thread::sleep(std::time::Duration::from_secs(2));
        let ltv_one = 0.70;
        let account_one = "account-1";
        let account_data_one = "70".as_bytes().to_vec();

        let ltv_two = 0.75;
        let account_two = "account-2";
        let account_data_two = "75".as_bytes().to_vec();

        let ltv_three = 0.80;
        let account_three = "account-3";
        let account_data_three = "80".as_bytes().to_vec();

        let ltv_four = 0.85;
        let account_four = "account-4";
        let account_data_four = "85".as_bytes().to_vec();

        put_obligation(
            &conn,
            ltv_one,
            account_one,
            &account_data_one[..],
            scraped_at_one,
        )
        .unwrap();
        put_obligation(
            &conn,
            ltv_two,
            account_two,
            &account_data_two[..],
            scraped_at_one,
        )
        .unwrap();
        put_obligation(
            &conn,
            ltv_three,
            account_three,
            &account_data_three[..],
            scraped_at_one,
        )
        .unwrap();
        put_obligation(
            &conn,
            ltv_four,
            account_four,
            &account_data_four[..],
            scraped_at_one,
        )
        .unwrap();

        let results =
            get_obligation(&conn, &ObligationMatcher::All, Some(LtvFilter::GE(0.70))).unwrap();
        assert_eq!(results.len(), 4);

        let results =
            get_obligation(&conn, &ObligationMatcher::All, Some(LtvFilter::GE(0.75))).unwrap();
        assert_eq!(results.len(), 3);

        let results =
            get_obligation(&conn, &ObligationMatcher::All, Some(LtvFilter::GE(0.80))).unwrap();
        assert_eq!(results.len(), 2);
        let results =
            get_obligation(&conn, &ObligationMatcher::All, Some(LtvFilter::GE(0.85))).unwrap();
        assert_eq!(results.len(), 1);

        let results =
            get_obligation(&conn, &ObligationMatcher::All, Some(LtvFilter::LE(0.85))).unwrap();
        assert_eq!(results.len(), 4);
        let results =
            get_obligation(&conn, &ObligationMatcher::All, Some(LtvFilter::LE(0.80))).unwrap();
        assert_eq!(results.len(), 3);
        let results =
            get_obligation(&conn, &ObligationMatcher::All, Some(LtvFilter::LE(0.75))).unwrap();
        assert_eq!(results.len(), 2);
        let results =
            get_obligation(&conn, &ObligationMatcher::All, Some(LtvFilter::LE(0.70))).unwrap();
        assert_eq!(results.len(), 1);

        let results =
            get_obligation(&conn, &ObligationMatcher::All, Some(LtvFilter::GT(0.85))).unwrap();
        assert_eq!(results.len(), 0);
        let results =
            get_obligation(&conn, &ObligationMatcher::All, Some(LtvFilter::GT(0.80))).unwrap();
        assert_eq!(results.len(), 1);
        let results =
            get_obligation(&conn, &ObligationMatcher::All, Some(LtvFilter::GT(0.75))).unwrap();
        assert_eq!(results.len(), 2);
        let results =
            get_obligation(&conn, &ObligationMatcher::All, Some(LtvFilter::GT(0.70))).unwrap();
        assert_eq!(results.len(), 3);
        let results =
            get_obligation(&conn, &ObligationMatcher::All, Some(LtvFilter::GT(0.69))).unwrap();
        assert_eq!(results.len(), 4);

        let results =
            get_obligation(&conn, &ObligationMatcher::All, Some(LtvFilter::LT(0.90))).unwrap();
        assert_eq!(results.len(), 4);
        let results =
            get_obligation(&conn, &ObligationMatcher::All, Some(LtvFilter::LT(0.85))).unwrap();
        assert_eq!(results.len(), 3);
        let results =
            get_obligation(&conn, &ObligationMatcher::All, Some(LtvFilter::LT(0.80))).unwrap();
        assert_eq!(results.len(), 2);
        let results =
            get_obligation(&conn, &ObligationMatcher::All, Some(LtvFilter::LT(0.75))).unwrap();
        assert_eq!(results.len(), 1);
        let results =
            get_obligation(&conn, &ObligationMatcher::All, Some(LtvFilter::LT(0.70))).unwrap();
        assert_eq!(results.len(), 0);
    }
}
