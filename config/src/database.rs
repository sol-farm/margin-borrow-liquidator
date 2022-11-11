use serde::{Deserialize, Serialize};

#[remain::sorted]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Database {
    /// pool size specific to the analytics service
    pub analytics_pool_size: u32,
    pub conn_url: String,
    /// pool size used by all other services, etc..
    pub pool_size: u32,
}
