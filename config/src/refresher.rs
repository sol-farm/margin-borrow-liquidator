//! configuration information for the refresher service
use serde::{Deserialize, Serialize};

#[remain::sorted]
#[derive(Clone, Default, Debug, Serialize, Deserialize)]
/// provides configuration options for the refresher service
pub struct Refresher {
    /// how often in seconds the refresh workloop should run
    pub frequency: u64,
    /// the maximum number of concurrent refreshes which can be running
    pub max_concurrency: u64,
    /// the max size of the database connection pool
    pub pool_size: u64,
}
