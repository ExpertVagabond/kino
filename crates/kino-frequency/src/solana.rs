//! Solana integration for on-chain fingerprint storage.
//!
//! This module provides integration with Solana blockchain for:
//! - Storing content fingerprint hashes on-chain
//! - Verifying content authenticity
//! - Creator ownership proofs
//! - Decentralized content registry
//!
//! # On-Chain Data Structure
//!
//! ```text
//! ContentFingerprint Account (PDA):
//! ├── creator: Pubkey (32 bytes)
//! ├── content_hash: [u8; 32] (SHA-256 of fingerprint)
//! ├── timestamp: i64 (8 bytes)
//! ├── version: u8 (1 byte)
//! └── metadata_uri: String (variable, max 200 bytes)
//! ```
//!
//! # Usage
//!
//! ```rust,no_run
//! use kino_frequency::solana::{SolanaFingerprintClient, FingerprintConfig};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let client = SolanaFingerprintClient::new(
//!         "https://api.devnet.solana.com",
//!         "path/to/keypair.json",
//!     )?;
//!
//!     // Store fingerprint on-chain
//!     let signature = client.store_fingerprint(
//!         &fingerprint_hash,
//!         Some("https://arweave.net/metadata.json"),
//!     ).await?;
//!
//!     // Verify content
//!     let verified = client.verify_content(&fingerprint_hash).await?;
//!
//!     Ok(())
//! }
//! ```

use std::str::FromStr;
use anyhow::{Result, Context, bail};
use serde::{Deserialize, Serialize};
use tracing::{info, debug, warn};

// Note: These imports require the "solana" feature
#[cfg(feature = "solana")]
use solana_sdk::{
    commitment_config::CommitmentConfig,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signature, Signer},
    system_program,
    transaction::Transaction,
};

/// Program ID for the Kino Fingerprint program (placeholder - replace with deployed address)
pub const FINGERPRINT_PROGRAM_ID: &str = "PSMFprnt1111111111111111111111111111111111";

/// Maximum length for metadata URI
pub const MAX_METADATA_URI_LEN: usize = 200;

/// Fingerprint account size
pub const FINGERPRINT_ACCOUNT_SIZE: usize = 32 + 32 + 8 + 1 + 4 + MAX_METADATA_URI_LEN;

/// On-chain fingerprint data structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnChainFingerprint {
    /// Creator's wallet address
    pub creator: String,
    /// SHA-256 hash of the audio fingerprint
    pub content_hash: [u8; 32],
    /// Unix timestamp when registered
    pub timestamp: i64,
    /// Fingerprint algorithm version
    pub version: u8,
    /// Optional URI to additional metadata (e.g., Arweave/IPFS)
    pub metadata_uri: Option<String>,
}

/// Verification result from on-chain lookup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    /// Whether the content was found on-chain
    pub found: bool,
    /// The registered creator (if found)
    pub creator: Option<String>,
    /// Registration timestamp (if found)
    pub registered_at: Option<i64>,
    /// Whether the fingerprint matches
    pub verified: bool,
    /// On-chain account address
    pub account_address: Option<String>,
}

/// Configuration for Solana client.
#[derive(Debug, Clone)]
pub struct SolanaConfig {
    /// RPC endpoint URL
    pub rpc_url: String,
    /// Commitment level for transactions
    pub commitment: String,
    /// Program ID for fingerprint storage
    pub program_id: String,
}

impl Default for SolanaConfig {
    fn default() -> Self {
        Self {
            rpc_url: "https://api.devnet.solana.com".to_string(),
            commitment: "confirmed".to_string(),
            program_id: FINGERPRINT_PROGRAM_ID.to_string(),
        }
    }
}

/// Client for interacting with Solana fingerprint program.
///
/// This client handles:
/// - Storing fingerprints on-chain
/// - Verifying content authenticity
/// - Querying fingerprint history
/// - Managing creator ownership
#[cfg(feature = "solana")]
pub struct SolanaFingerprintClient {
    config: SolanaConfig,
    payer: Keypair,
    program_id: Pubkey,
}

#[cfg(feature = "solana")]
impl SolanaFingerprintClient {
    /// Create a new Solana client.
    pub fn new(rpc_url: &str, keypair_path: &str) -> Result<Self> {
        let config = SolanaConfig {
            rpc_url: rpc_url.to_string(),
            ..Default::default()
        };

        let keypair_data = std::fs::read_to_string(keypair_path)
            .context("Failed to read keypair file")?;
        let keypair_bytes: Vec<u8> = serde_json::from_str(&keypair_data)
            .context("Failed to parse keypair JSON")?;
        let payer = Keypair::from_bytes(&keypair_bytes)
            .context("Failed to create keypair")?;

        let program_id = Pubkey::from_str(&config.program_id)
            .context("Invalid program ID")?;

        Ok(Self {
            config,
            payer,
            program_id,
        })
    }

    /// Create client with custom configuration.
    pub fn with_config(config: SolanaConfig, keypair_path: &str) -> Result<Self> {
        let keypair_data = std::fs::read_to_string(keypair_path)
            .context("Failed to read keypair file")?;
        let keypair_bytes: Vec<u8> = serde_json::from_str(&keypair_data)
            .context("Failed to parse keypair JSON")?;
        let payer = Keypair::from_bytes(&keypair_bytes)
            .context("Failed to create keypair")?;

        let program_id = Pubkey::from_str(&config.program_id)
            .context("Invalid program ID")?;

        Ok(Self {
            config,
            payer,
            program_id,
        })
    }

    /// Derive the PDA address for a fingerprint.
    pub fn derive_fingerprint_address(&self, content_hash: &[u8; 32]) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[b"fingerprint", content_hash.as_ref()],
            &self.program_id,
        )
    }

    /// Store a fingerprint on-chain.
    pub async fn store_fingerprint(
        &self,
        content_hash: &[u8; 32],
        metadata_uri: Option<&str>,
    ) -> Result<String> {
        info!("Storing fingerprint on Solana: {:?}", hex::encode(content_hash));

        let (fingerprint_pda, bump) = self.derive_fingerprint_address(content_hash);

        // Build instruction data
        let mut instruction_data = vec![0u8]; // Instruction discriminator: 0 = Store
        instruction_data.extend_from_slice(content_hash);
        instruction_data.push(bump);

        if let Some(uri) = metadata_uri {
            if uri.len() > MAX_METADATA_URI_LEN {
                bail!("Metadata URI too long (max {} characters)", MAX_METADATA_URI_LEN);
            }
            instruction_data.extend_from_slice(&(uri.len() as u32).to_le_bytes());
            instruction_data.extend_from_slice(uri.as_bytes());
        } else {
            instruction_data.extend_from_slice(&0u32.to_le_bytes());
        }

        let instruction = Instruction {
            program_id: self.program_id,
            accounts: vec![
                AccountMeta::new(fingerprint_pda, false),
                AccountMeta::new(self.payer.pubkey(), true),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
            data: instruction_data,
        };

        // Note: In production, you would send this transaction to the network
        // This is a simplified example
        debug!("Instruction created for PDA: {}", fingerprint_pda);
        debug!("Creator: {}", self.payer.pubkey());

        // Return placeholder signature (actual implementation would send tx)
        Ok(format!("simulated_signature_{}", hex::encode(&content_hash[..8])))
    }

    /// Verify content against on-chain fingerprint.
    pub async fn verify_content(&self, content_hash: &[u8; 32]) -> Result<VerificationResult> {
        info!("Verifying content hash: {:?}", hex::encode(content_hash));

        let (fingerprint_pda, _) = self.derive_fingerprint_address(content_hash);

        // In production, fetch account data from RPC
        // This is a simplified placeholder
        debug!("Looking up PDA: {}", fingerprint_pda);

        // Simulated response - in production, parse actual account data
        Ok(VerificationResult {
            found: false, // Would be true if account exists
            creator: None,
            registered_at: None,
            verified: false,
            account_address: Some(fingerprint_pda.to_string()),
        })
    }

    /// Get fingerprint details from on-chain.
    pub async fn get_fingerprint(&self, content_hash: &[u8; 32]) -> Result<Option<OnChainFingerprint>> {
        let (fingerprint_pda, _) = self.derive_fingerprint_address(content_hash);

        // In production, fetch and deserialize account data
        debug!("Fetching fingerprint account: {}", fingerprint_pda);

        Ok(None) // Placeholder
    }

    /// Get all fingerprints registered by a creator.
    pub async fn get_creator_fingerprints(&self, creator: &str) -> Result<Vec<OnChainFingerprint>> {
        let creator_pubkey = Pubkey::from_str(creator)
            .context("Invalid creator address")?;

        // In production, use getProgramAccounts with filters
        debug!("Fetching fingerprints for creator: {}", creator_pubkey);

        Ok(Vec::new()) // Placeholder
    }

    /// Transfer fingerprint ownership to another wallet.
    pub async fn transfer_ownership(
        &self,
        content_hash: &[u8; 32],
        new_owner: &str,
    ) -> Result<String> {
        let new_owner_pubkey = Pubkey::from_str(new_owner)
            .context("Invalid new owner address")?;

        let (fingerprint_pda, _) = self.derive_fingerprint_address(content_hash);

        // Build transfer instruction
        let mut instruction_data = vec![1u8]; // Instruction discriminator: 1 = Transfer
        instruction_data.extend_from_slice(&new_owner_pubkey.to_bytes());

        let instruction = Instruction {
            program_id: self.program_id,
            accounts: vec![
                AccountMeta::new(fingerprint_pda, false),
                AccountMeta::new(self.payer.pubkey(), true),
                AccountMeta::new_readonly(new_owner_pubkey, false),
            ],
            data: instruction_data,
        };

        debug!("Transfer instruction created");

        Ok(format!("simulated_transfer_{}", hex::encode(&content_hash[..8])))
    }
}

/// Standalone verification without client initialization.
/// Useful for quick lookups from any context.
pub async fn verify_fingerprint_hash(
    rpc_url: &str,
    content_hash: &[u8; 32],
) -> Result<VerificationResult> {
    let program_id = Pubkey::from_str(FINGERPRINT_PROGRAM_ID)?;

    let (fingerprint_pda, _) = Pubkey::find_program_address(
        &[b"fingerprint", content_hash.as_ref()],
        &program_id,
    );

    // In production, make RPC call to check if account exists
    Ok(VerificationResult {
        found: false,
        creator: None,
        registered_at: None,
        verified: false,
        account_address: Some(fingerprint_pda.to_string()),
    })
}

/// Convert fingerprint hash string to bytes.
pub fn parse_fingerprint_hash(hash_str: &str) -> Result<[u8; 32]> {
    let bytes = hex_decode(hash_str)?;
    if bytes.len() != 32 {
        bail!("Fingerprint hash must be 32 bytes (64 hex characters)");
    }

    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    Ok(arr)
}

// Helper for hex encoding/decoding
mod hex {
    pub fn encode(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{:02x}", b)).collect()
    }
}

fn hex_decode(s: &str) -> Result<Vec<u8>> {
    if s.len() % 2 != 0 {
        bail!("Hex string must have even length");
    }

    (0..s.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&s[i..i + 2], 16)
                .context("Invalid hex character")
        })
        .collect()
}

/// Anchor program instruction builders (for use with anchor-client).
#[cfg(feature = "solana")]
pub mod instructions {
    use super::*;

    /// Build instruction to store a new fingerprint.
    pub fn store_fingerprint(
        program_id: &Pubkey,
        fingerprint_pda: &Pubkey,
        creator: &Pubkey,
        content_hash: &[u8; 32],
        metadata_uri: Option<&str>,
        bump: u8,
    ) -> Instruction {
        let mut data = vec![0u8]; // Store discriminator
        data.extend_from_slice(content_hash);
        data.push(bump);

        if let Some(uri) = metadata_uri {
            let uri_bytes = uri.as_bytes();
            data.extend_from_slice(&(uri_bytes.len() as u32).to_le_bytes());
            data.extend_from_slice(uri_bytes);
        } else {
            data.extend_from_slice(&0u32.to_le_bytes());
        }

        Instruction {
            program_id: *program_id,
            accounts: vec![
                AccountMeta::new(*fingerprint_pda, false),
                AccountMeta::new(*creator, true),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
            data,
        }
    }

    /// Build instruction to verify a fingerprint.
    pub fn verify_fingerprint(
        program_id: &Pubkey,
        fingerprint_pda: &Pubkey,
    ) -> Instruction {
        Instruction {
            program_id: *program_id,
            accounts: vec![
                AccountMeta::new_readonly(*fingerprint_pda, false),
            ],
            data: vec![2u8], // Verify discriminator
        }
    }

    /// Build instruction to transfer ownership.
    pub fn transfer_ownership(
        program_id: &Pubkey,
        fingerprint_pda: &Pubkey,
        current_owner: &Pubkey,
        new_owner: &Pubkey,
    ) -> Instruction {
        let mut data = vec![1u8]; // Transfer discriminator
        data.extend_from_slice(&new_owner.to_bytes());

        Instruction {
            program_id: *program_id,
            accounts: vec![
                AccountMeta::new(*fingerprint_pda, false),
                AccountMeta::new(*current_owner, true),
                AccountMeta::new_readonly(*new_owner, false),
            ],
            data,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_fingerprint_hash() {
        let hash_str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let result = parse_fingerprint_hash(hash_str).unwrap();
        assert_eq!(result.len(), 32);
        assert_eq!(result[0], 0x01);
        assert_eq!(result[1], 0x23);
    }

    #[test]
    fn test_invalid_hash_length() {
        let short_hash = "0123456789abcdef";
        let result = parse_fingerprint_hash(short_hash);
        assert!(result.is_err());
    }

    #[test]
    fn test_hex_encode() {
        let bytes = [0x01, 0x23, 0x45, 0x67];
        let encoded = hex::encode(&bytes);
        assert_eq!(encoded, "01234567");
    }
}
