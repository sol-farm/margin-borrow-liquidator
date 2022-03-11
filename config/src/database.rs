use serde::{Deserialize, Serialize};

#[remain::sorted]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Database {
    pub analytics_pool_size: u32,
    pub conn_url: String,
    /// used by the price api
    pub pool_size: u32,
}
