//! the same request_builder from https://github.com/project-serum/anchor with minor modifications

use anchor_client::anchor_lang::InstructionData;
use anchor_client::anchor_lang::ToAccountMetas;
use anchor_client::solana_client::rpc_client::RpcClient;
use anchor_client::solana_client::rpc_config::RpcSendTransactionConfig;

use anchor_client::solana_sdk::commitment_config::CommitmentConfig;

use anchor_client::solana_sdk::instruction::AccountMeta;
use anchor_client::solana_sdk::instruction::Instruction;
use anchor_client::solana_sdk::pubkey::Pubkey;

use anchor_client::solana_sdk::signature::Signature;
use anchor_client::solana_sdk::signature::Signer;

use anchor_client::solana_sdk::transaction::Transaction;

use anyhow::{anyhow, Result};

use std::sync::Arc;

#[derive(Clone)]
pub struct RequestBuilder<'a> {
    /// address of the program being the request builder is being used for
    pub program_id: Pubkey,
    pub payer: &'a dyn Signer,
    pub signers: Vec<&'a dyn Signer>,
    pub rpc_client: Arc<RpcClient>,
    pub instructions: Vec<Instruction>,
    pub accounts: Vec<AccountMeta>,
    pub instruction_data: Option<Vec<u8>>,
}

impl<'a> RequestBuilder<'a> {
    /// returns a new request builder using an rpc with a customized timeout
    /// of 60 seconds, and a commitment of finalized
    pub fn new(
        program_id: Pubkey, 
        cluster_url: String, 
        payer: &'a dyn Signer,
        timeout: Option<std::time::Duration>,
        commitment: Option<CommitmentConfig>,
    ) -> Self {
        let rpc_client = if let (Some(timeout), Some(commitment)) = (timeout, commitment) {
            Arc::new(RpcClient::new_with_timeout_and_commitment(
                cluster_url,
                timeout,
                commitment,
            ))
        } else {
            Arc::new(RpcClient::new_with_timeout(
                cluster_url,
                std::time::Duration::from_secs(60),
            ))
        };
        Self {
            program_id,
            payer,
            signers: vec![],
            // we'll always have at least 1 instruction, so preallocate 2
            instructions: vec![],
            accounts: vec![],
            instruction_data: None,
            rpc_client,
        }
    }
    /// returns a pointer to the RpcClient object
    pub fn rpc(&'a self) -> &'a Arc<RpcClient> {
        &self.rpc_client
    }
    /// overrides the existing payer
    pub fn payer(mut self, payer: &'a dyn Signer) -> Self {
        self.payer = payer;
        self
    }
    pub fn instruction(mut self, ix: Instruction) -> Self {
        self.instructions.push(ix);
        self
    }
    pub fn accounts(mut self, accounts: impl ToAccountMetas) -> Self {
        let mut metas = accounts.to_account_metas(None);
        self.accounts.append(&mut metas);
        self
    }
    pub fn args(mut self, args: impl InstructionData) -> Self {
        self.instruction_data = Some(args.data());
        self
    }
    /// adds an additional signer
    pub fn signer(mut self, signer: &'a dyn Signer) -> Self {
        self.signers.push(signer);
        self
    }
    /// sends the current transaction with the option to skip preflight, or 
    /// display a spinner while waiting for confirmation
    pub fn send(&self, skip_preflight: bool, spinner: bool) -> Result<Signature> {
        let tx = self.create_tx()?;

        if skip_preflight {
            Ok(self.rpc_client.send_transaction_with_config(
                &tx,
                RpcSendTransactionConfig {
                    skip_preflight: true,
                    ..Default::default()
                },
            )?)
        } else if spinner {
            Ok(self
                .rpc_client
                .send_and_confirm_transaction_with_spinner(&tx)?)
        } else {
            Ok(self.rpc_client.send_and_confirm_transaction(&tx)?)
        }
    }
    /// sends the current transaction with an exponential backoff
    pub fn send_with_backoff(&self) -> Result<Signature> {
        let tx = self.create_tx()?;
        let do_fn = || -> Result<Signature> {
            match self.rpc_client.send_and_confirm_transaction(&tx) {
                Ok(sig) => Ok(sig),
                Err(err) => Err(anyhow!("{:#?}", err)),
            }
        };

        crate::sender::do_with_exponential_backoff(do_fn)
    }
    /// takes the current data within the request builder
    /// and constructs a signed transaction ready for broadcasting
    pub fn create_tx(&self) -> Result<Transaction> {
        let instructions = if let Some(ix_data) = &self.instruction_data {
            let mut instructions = vec![Instruction {
                program_id: self.program_id,
                data: ix_data.clone(),
                accounts: self.accounts.clone(),
            }];
            instructions.extend_from_slice(&self.instructions[..]);
            instructions
        } else {
            self.instructions.clone()
        };
        let blockhash = self.rpc_client.get_latest_blockhash()?;
        let mut signers = vec![&*self.payer];
        for signer in self.signers.iter() {
            signers.push(&**signer);
        }
        let tx = Transaction::new_signed_with_payer(
            &instructions,
            Some(&self.payer.pubkey()),
            &signers,
            blockhash,
        );
        Ok(tx)
    }
    /// returns a new instance of the request builder, with all
    /// instruction and account data reset. use this when needing
    /// to create multiple new transactions for the same program.
    pub fn request(&self) -> RequestBuilder {
        self.clone()
    }
    /// resets any transaction related data, useful for clearing the request
    /// builder in-between different transactions, optionally setting a new
    /// program_id for which the request builder will construct transactions for
    pub fn reset(&mut self, program_id: Option<Pubkey>) {
        self.signers = vec![];
        self.instructions = vec![];
        self.accounts = vec![];
        self.instruction_data = None;
        if let Some(program_id) = program_id {
            self.program_id = program_id;
        }
    }
}
