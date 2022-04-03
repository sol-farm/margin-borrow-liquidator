//! provides utilities for sending transactions

use anyhow::{anyhow, Result};
use retry::{delay::Exponential, Error as RetryError};
use solana_sdk::signature::Signature;

/// runs the provided function with an exponential backoff starting at 100ms, with a scaling factor of 2
/// runs up to 3 times before failing
pub fn do_with_exponential_backoff(do_fn: impl Fn() -> Result<Signature>) -> Result<Signature> {
    match retry::retry(
        // 100, 200, 400
        Exponential::from_millis_with_factor(100, 2.0).take(3),
        do_fn,
    ) {
        Ok(sig) => Ok(sig),
        Err(err) => match err {
            RetryError::Operation {
                error,
                total_delay: _,
                tries: _,
            } => Err(error),
            RetryError::Internal(msg) => Err(anyhow!("{:#?}", msg)),
        },
    }
}
