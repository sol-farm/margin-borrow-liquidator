//! provides the main liquidator service component, responsible for executing liquidations and collecting bounties.
//! the "simple" liquidator is a rudimentary liquidator that requires the person running the liquidator to have all required
//! funds on hand to pay off all required debt. 

pub mod simple;
pub mod instructions;
