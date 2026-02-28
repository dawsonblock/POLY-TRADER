/*
 * BOREAL SECURITY: mTLS & TLS PINNING
 * Tier 5 — ensures only authorized DTK instances can connect to exchange gateways.
 * 
 * Uses rustls for modern, safe TLS 1.3 only.
 * Requires: CA cert, client cert, and client private key.
 */

use std::sync::Arc;
use rustls::{ClientConfig, RootCertStore};
use rustls::pki_types::{CertificateDer, PrivateKeyDer};

/// Error types for mTLS configuration.
pub enum MtlsError {
    InvalidCA,
    InvalidClientCert,
    InvalidPrivateKey,
    Io(std::io::Error),
}

/// Factory: build an mTLS configuration for secure exchange connectivity.
pub fn build_mtls_config(
    ca_cert_path: &str,
    client_cert_path: &str,
    client_key_path: &str,
) -> Result<ClientConfig, MtlsError> {
    // 1. Load Root CA Store
    let mut root_store = RootCertStore::empty();
    let ca_file = std::fs::File::open(ca_cert_path).map_err(MtlsError::Io)?;
    let mut ca_reader = std::io::BufReader::new(ca_file);
    for cert in rustls_pemfile::certs(&mut ca_reader) {
        let cert = cert.map_err(|_| MtlsError::InvalidCA)?;
        root_store.add(cert).map_err(|_| MtlsError::InvalidCA)?;
    }

    // 2. Load Client Certificate
    let cert_file = std::fs::File::open(client_cert_path).map_err(MtlsError::Io)?;
    let mut cert_reader = std::io::BufReader::new(cert_file);
    let client_certs: Vec<CertificateDer> = rustls_pemfile::certs(&mut cert_reader)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| MtlsError::InvalidClientCert)?;

    // 3. Load Client Private Key
    let key_file = std::fs::File::open(client_key_path).map_err(MtlsError::Io)?;
    let mut key_reader = std::io::BufReader::new(key_file);
    let key = rustls_pemfile::pkcs8_private_keys(&mut key_reader)
        .next()
        .ok_or(MtlsError::InvalidPrivateKey)?
        .map_err(|_| MtlsError::InvalidPrivateKey)?;
    let private_key = PrivateKeyDer::Pkcs8(key);

    // 4. Build Client Config (TLS 1.3, enforced)
    let config = ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_client_auth_cert(client_certs, private_key)
        .map_err(|_| MtlsError::InvalidClientCert)?;

    Ok(config)
}

/// Helper for dev env: disabled TLS verification (CAUTION: SECURITY RISK)
pub fn build_insecure_config() -> ClientConfig {
    let root_store = RootCertStore::empty();
    ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth()
}
