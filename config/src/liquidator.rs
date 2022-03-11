use anchor_lang::prelude::*;
use serde::{Deserialize, Serialize};

#[remain::sorted]
#[derive(Clone, Default, Debug, Serialize, Deserialize)]
/// provides configuration options for the liquidator service
pub struct Liquidator {
    /// how often in seconds the liquidator workloop should run
    pub frequency: u64,
    /// the maximum number of concurrent tasks executable by the liquidator,
    /// this includes liquidating a position, checking if a position can be liquidated, etc..
    pub max_concurrency: u64,
    /// the minimum ltv to use for filtering obligations from the database to check for liquidations
    /// if 0, no ltv filtering is done, otherwise uses a greater than or equal to filter method.
    /// this means if you specify 0.7, obligations with an ltv greater than or equal to 0.7 (70%) will be returned
    pub min_ltv: f64,
}
