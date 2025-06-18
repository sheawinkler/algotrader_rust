use solana_sdk::{
    commitment_config::CommitmentConfig,
    instruction::Instruction,
    message::Message,
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
    transaction::Transaction,
};
use solana_client::{
    rpc_client::RpcClient,
    rpc_config::RpcSendTransactionConfig,
};
use anyhow::{Result, anyhow};
use std::sync::Arc;

/// Builder for Solana transactions
pub struct TransactionBuilder {
    instructions: Vec<Instruction>,
    signers: Vec<Arc<Keypair>>,
    fee_payer: Option<Pubkey>,
    recent_blockhash: Option<String>,
}

impl Default for TransactionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl TransactionBuilder {
    /// Create a new transaction builder
    pub fn new() -> Self {
        Self {
            instructions: Vec::new(),
            signers: Vec::new(),
            fee_payer: None,
            recent_blockhash: None,
        }
    }

    /// Add an instruction to the transaction
    pub fn add_instruction(mut self, instruction: Instruction) -> Self {
        self.instructions.push(instruction);
        self
    }

    /// Add multiple instructions to the transaction
    pub fn add_instructions(mut self, instructions: Vec<Instruction>) -> Self {
        self.instructions.extend(instructions);
        self
    }

    /// Add a signer to the transaction
    pub fn add_signer(mut self, signer: Arc<Keypair>) -> Self {
        self.signers.push(signer);
        self
    }

    /// Set the fee payer for the transaction
    pub fn fee_payer(mut self, fee_payer: Pubkey) -> Self {
        self.fee_payer = Some(fee_payer);
        self
    }

    /// Set the recent blockhash for the transaction
    pub fn recent_blockhash(mut self, recent_blockhash: String) -> Self {
        self.recent_blockhash = Some(recent_blockhash);
        self
    }

    /// Build an unsigned transaction
    pub fn build_unsigned(self) -> Result<Transaction> {
        if self.instructions.is_empty() {
            return Err(anyhow!("No instructions provided"));
        }

        let message = Message::new(&self.instructions, self.fee_payer.as_ref());
        let mut tx = Transaction::new_unsigned(message);

        if let Some(recent_blockhash) = self.recent_blockhash {
            let blockhash = recent_blockhash.parse().map_err(|_| anyhow!("Invalid recent blockhash"))?;
            let signers: Vec<&Keypair> = self.signers.iter().map(|s| s.as_ref()).collect();
            tx.try_partial_sign(&signers, blockhash)?;
        }

        Ok(tx)
    }

    /// Build and sign a transaction
    pub fn build(mut self, client: &RpcClient) -> Result<Transaction> {
        if self.instructions.is_empty() {
            return Err(anyhow!("No instructions provided"));
        }

        // If no fee payer is set, use the first signer
        if self.fee_payer.is_none() && !self.signers.is_empty() {
            self.fee_payer = Some(self.signers[0].pubkey());
        }

        let recent_blockhash = if let Some(blockhash) = self.recent_blockhash {
            blockhash
        } else {
            // Get recent blockhash if not provided
            client.get_latest_blockhash()?.to_string()
        };

        let message = Message::new(&self.instructions, self.fee_payer.as_ref());
        let mut tx = Transaction::new_unsigned(message);

        // Sign the transaction with all signers
        let signer_refs: Vec<&Keypair> = self.signers.iter().map(|s| s.as_ref()).collect();
        let blockhash = recent_blockhash.parse()?;
        tx.try_sign(&signer_refs, blockhash)?;

        Ok(tx)
    }

    /// Build, sign, and send a transaction
    pub async fn send(
        self,
        client: &RpcClient,
        skip_preflight: bool,
    ) -> Result<String> {
        let tx = self.build(client)?;
        
        let config = RpcSendTransactionConfig {
            skip_preflight,
            preflight_commitment: Some(CommitmentConfig::confirmed().commitment),
            encoding: None,
            max_retries: Some(3),
            min_context_slot: None,
        };

        let signature = client.send_transaction_with_config(&tx, config)?;
        Ok(signature.to_string())
    }
}

/// Helper functions for common transaction operations
pub struct TransactionHelper;

impl TransactionHelper {
    /// Calculate the transaction fee for a given set of instructions
    pub fn calculate_fee(
        client: &RpcClient,
        instructions: &[Instruction],
        signer_count: usize,
    ) -> Result<u64> {
        let message = Message::new(instructions, None);
        let blockhash = client.get_latest_blockhash()?;
        
        // Get fee for the message
        let fee = client.get_fee_for_message(&message)?;
        
        // Each signer adds a signature fee (approximate)
        let signature_fee = signer_count as u64 * 5000; // Approximate fee per signature
        
        Ok(fee * signer_count as u64 + signature_fee)
    }
    
    /// Check if a transaction has been confirmed
    pub fn is_transaction_confirmed(
        client: &RpcClient,
        signature: &str,
    ) -> Result<bool> {
        let status = client.get_signature_statuses(&[signature.parse()?])?;
        Ok(status.value[0].as_ref().map_or(false, |s| s.confirmations.is_some()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::system_instruction;
    use std::str::FromStr;

    #[test]
    fn test_transaction_builder() {
        let from_pubkey = Pubkey::new_unique();
        let to_pubkey = Pubkey::new_unique();
        let amount = 1_000_000; // 1 SOL in lamports

        let instruction = system_instruction::transfer(&from_pubkey, &to_pubkey, amount);
        
        let builder = TransactionBuilder::new()
            .add_instruction(instruction)
            .fee_payer(from_pubkey);
            
        assert_eq!(builder.instructions.len(), 1);
        assert_eq!(builder.fee_payer, Some(from_pubkey));
    }
    
    #[test]
    fn test_transaction_builder_no_instructions() {
        let builder = TransactionBuilder::new();
        assert!(builder.build_unsigned().is_err());
    }
}
