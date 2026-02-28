/*
 * BOREAL SECURITY: ORDER SIGNER ABSTRACTION
 * Tier 5 — separates signing from execution.
 * 
 * In development: SoftwareSigner (key from env).
 * In production: HsmSigner (YubiHSM2 / AWS CloudHSM / Nitro Enclave).
 * Switching is zero-code — set SIGNER_BACKEND=hsm.
 */

use crate::bcore::features::fixed_point::Fixed;

/// The intent that needs signing for broadcast to exchange.
#[derive(Debug, Clone, Copy)]
pub struct OrderIntent {
    pub side:  u8,    // 1 = buy, 2 = sell
    pub price: Fixed,
    pub size:  Fixed,
    pub nonce: u64,   // Monotonic nonce — replay protection
}

/// Abstract signing interface. Implementations may be hardware-backed.
pub trait OrderSigner: Send + Sync {
    fn sign(&self, intent: &OrderIntent) -> Result<Vec<u8>, SignerError>;
    fn backend_name(&self) -> &'static str;
}

#[derive(Debug)]
pub enum SignerError {
    KeyNotFound,
    HsmCommunicationFailed(String),
    InvalidIntent,
}

impl std::fmt::Display for SignerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::KeyNotFound => write!(f, "Signing key not found"),
            Self::HsmCommunicationFailed(e) => write!(f, "HSM error: {e}"),
            Self::InvalidIntent => write!(f, "Invalid order intent"),
        }
    }
}

/// DEV ONLY: signs using a private key from the environment.
/// WARNING: Key is in process memory. Never use with real capital.
pub struct SoftwareSigner {
    /// Hex private key read from PRIVATE_KEY env var at startup
    _key_bytes: Vec<u8>,
}

impl SoftwareSigner {
    pub fn from_env() -> Result<Self, SignerError> {
        let key_hex = std::env::var("PRIVATE_KEY").map_err(|_| SignerError::KeyNotFound)?;
        let key_bytes = hex::decode(key_hex.trim_start_matches("0x"))
            .map_err(|_| SignerError::KeyNotFound)?;
        Ok(Self { _key_bytes: key_bytes })
    }
}

impl OrderSigner for SoftwareSigner {
    fn sign(&self, intent: &OrderIntent) -> Result<Vec<u8>, SignerError> {
        // Stub: in production, compute EIP-712 typed data hash + ECDSA sign
        // For now: return deterministic dummy signature based on intent bytes
        let mut sig = vec![0u8; 65];
        sig[0] = intent.side;
        sig[1..9].copy_from_slice(&intent.nonce.to_le_bytes());
        Ok(sig)
    }
    fn backend_name(&self) -> &'static str { "software (dev-only)" }
}

/// PRODUCTION STUB: HSM-backed signer interface.
/// Implement by wiring to yubihsm crate or AWS CloudHSM SDK.
pub struct HsmSigner;

impl OrderSigner for HsmSigner {
    fn sign(&self, _intent: &OrderIntent) -> Result<Vec<u8>, SignerError> {
        Err(SignerError::HsmCommunicationFailed(
            "HsmSigner not implemented — wire to YubiHSM2 or Nitro Enclave".into()
        ))
    }
    fn backend_name(&self) -> &'static str { "hsm (stub)" }
}

/// Factory: select signer based on SIGNER_BACKEND env var.
pub fn build_signer() -> Box<dyn OrderSigner> {
    match std::env::var("SIGNER_BACKEND").as_deref() {
        Ok("hsm") => Box::new(HsmSigner),
        _         => Box::new(SoftwareSigner::from_env().unwrap_or_else(|e| {
            eprintln!("[SIGNER][WARN] {e}. Signing will be no-op.");
            SoftwareSigner { _key_bytes: vec![] }
        })),
    }
}
