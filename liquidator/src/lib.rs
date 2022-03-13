//! provides the main liquidator service component, responsible for executing liquidations and collecting bounties.
//! the "simple" liquidator is a rudimentary liquidator that requires the person running the liquidator to have all required
//! funds on hand to pay off all required debt.

use tulipv2_sdk_common::math::{decimal::Decimal, uint::U192};

pub mod instructions;
pub mod refresher;
pub mod simple;

/// the minimum ltv used as the threshold at which liquidation occurs
pub const MIN_LTV: Decimal = Decimal(U192([600000000000000000, 0, 0]));

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_dec() {
        let dec = Decimal::from_percent(60);
        // uncomment to print the array that we use to derive the constant decimal
        //println!("{:?}", dec.0.0);
        assert_eq!(dec, MIN_LTV);
    }
}
