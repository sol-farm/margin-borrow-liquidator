use into_query::IntoQuery;

#[derive(IntoQuery, Default)]
#[table_name = "obligations"]
pub struct FindObligation {
    pub account: Option<Vec<String>>,
}

#[derive(IntoQuery, Default)]
#[table_name = "price_feeds"]
pub struct FindPriceFeed {
    pub token_mint: Option<Vec<String>>,
    pub price_account: Option<Vec<String>>,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ObligationMatcher {
    Account(Vec<String>),
    /// return all records
    All,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum PriceFeedMatcher {
    TokenMint(Vec<String>),
    PriceAccount(Vec<String>),
    /// return all records
    All,
}

impl ObligationMatcher {
    /// returns an instance of the obligation matcher
    pub fn to_filter(&self) -> FindObligation {
        let mut ft = FindObligation::default();
        match self {
            ObligationMatcher::Account(acct) => {
                ft.account = Some(acct.clone());
            }
            ObligationMatcher::All => (),
        }
        ft
    }
}

impl PriceFeedMatcher {
    /// returns an instance of the deposit tracking filter
    pub fn to_filter(&self) -> FindPriceFeed {
        let mut ft = FindPriceFeed::default();
        match self {
            PriceFeedMatcher::TokenMint(tkn_mint) => {
                ft.token_mint = Some(tkn_mint.clone());
            }
            PriceFeedMatcher::PriceAccount(accts) => {
                ft.price_account = Some(accts.clone());
            }
            PriceFeedMatcher::All => (),
        }
        ft
    }
}

#[derive(Clone, Copy, Debug)]
/// helper type used for applying additional query filters
/// when searching for obligations
pub enum LtvFilter {
    /// filters obligations with an ltv greater than or equal to the given value
    GE(f64),
    /// filters obligations with an ltv less than or equal to the given value
    LE(f64),
    /// filters obligations with an ltv greater than the given value
    GT(f64),
    /// filters obligations with an ltv less than the given value
    LT(f64),
}
