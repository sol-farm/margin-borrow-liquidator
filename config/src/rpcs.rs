use crate::Configuration;
use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::{pubkey::Pubkey, signer::Signer};
use std::sync::Arc;

#[remain::sorted]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RPCs {
    pub failover_endpoints: Vec<RPCEndpoint>,
    pub primary_endpoint: RPCEndpoint,
}

#[remain::sorted]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RPCEndpoint {
    pub http_url: String,
    pub ws_url: String,
}

impl Configuration {
    /// returns a request builder using the primary http rpc endpoint
    pub fn request_builder<'a>(
        &self,
        program_id: Pubkey,
        payer: &'a dyn Signer,
        timeout: Option<std::time::Duration>,
        commitment: Option<CommitmentConfig>,
    ) -> rpc_utils::request_builder::RequestBuilder<'a> {
        rpc_utils::request_builder::RequestBuilder::new(
            program_id,
            self.rpc_endpoints.primary_endpoint.http_url.clone(),
            payer,
            timeout,
            commitment,
        )
    }
    // returns the primary rpc provider
    pub fn get_rpc_client(&self, ws: bool, commitment: Option<CommitmentConfig>) -> RpcClient {
        if !ws {
            match commitment {
                Some(commitment) => {
                    return RpcClient::new_with_commitment(
                        self.rpc_endpoints.primary_endpoint.http_url.clone(),
                        commitment,
                    );
                }
                None => {
                    return RpcClient::new_with_commitment(
                        self.rpc_endpoints.primary_endpoint.http_url.clone(),
                        CommitmentConfig::confirmed(),
                    );
                }
            }
        }
        match commitment {
            Some(commitment) => RpcClient::new_with_commitment(
                self.rpc_endpoints.primary_endpoint.ws_url.clone(),
                commitment,
            ),
            None => RpcClient::new_with_commitment(
                self.rpc_endpoints.primary_endpoint.ws_url.clone(),
                CommitmentConfig::confirmed(),
            ),
        }
    }
    // returns a vector of Clients for the failover
    // rpc clients in the order they are declared in in the config file
    pub fn get_rpc_failover_clients(
        &self,
        ws: bool,
        commitment: Option<CommitmentConfig>,
    ) -> Vec<Arc<RpcClient>> {
        let mut clients = Vec::with_capacity(self.rpc_endpoints.failover_endpoints.len());
        for failover in self.rpc_endpoints.failover_endpoints.iter() {
            if !ws {
                match commitment {
                    Some(commitment) => clients.push(Arc::new(RpcClient::new_with_commitment(
                        failover.http_url.clone(),
                        commitment,
                    ))),
                    None => clients.push(Arc::new(RpcClient::new_with_commitment(
                        failover.http_url.clone(),
                        CommitmentConfig::confirmed(),
                    ))),
                }
            } else {
                match commitment {
                    Some(commitment) => clients.push(Arc::new(RpcClient::new_with_commitment(
                        failover.ws_url.clone(),
                        commitment,
                    ))),
                    None => clients.push(Arc::new(RpcClient::new_with_commitment(
                        failover.ws_url.clone(),
                        CommitmentConfig::confirmed(),
                    ))),
                }
            }
        }
        clients
    }
}
