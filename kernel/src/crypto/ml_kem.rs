//! ML-KEM-768 (FIPS 203) — module-lattice key encapsulation.
//!
//! When `pq-crypto` is enabled this wraps `oqs::kem::Kem` with
//! `Algorithm::MlKem768`. Otherwise it exposes stub newtypes so the rest of
//! the kernel can reference the API without a libOQS dependency on CI.

use anyhow::Result;

#[cfg(feature = "pq-crypto")]
use oqs::kem::{Algorithm, Kem};

#[derive(Debug, Clone)]
pub struct PublicKey(pub Vec<u8>);

#[derive(Debug, Clone)]
pub struct SecretKey(pub Vec<u8>);

#[derive(Debug, Clone)]
pub struct Ciphertext(pub Vec<u8>);

#[derive(Debug, Clone)]
pub struct SharedSecret(pub Vec<u8>);

pub struct MlKemKeyPair {
    pub public: PublicKey,
    pub secret: SecretKey,
}

#[cfg(feature = "pq-crypto")]
impl MlKemKeyPair {
    pub fn generate() -> Result<Self> {
        let kem = Kem::new(Algorithm::MlKem768)?;
        let (pk, sk) = kem.keypair()?;
        Ok(Self {
            public: PublicKey(pk.into_vec()),
            secret: SecretKey(sk.into_vec()),
        })
    }
}

#[cfg(not(feature = "pq-crypto"))]
impl MlKemKeyPair {
    pub fn generate() -> Result<Self> {
        Ok(Self {
            public: PublicKey(Vec::new()),
            secret: SecretKey(Vec::new()),
        })
    }
}

#[cfg(feature = "pq-crypto")]
pub fn encapsulate(pk: &PublicKey) -> Result<(Ciphertext, SharedSecret)> {
    let kem = Kem::new(Algorithm::MlKem768)?;
    let pk_ref = kem
        .public_key_from_bytes(&pk.0)
        .ok_or_else(|| anyhow::anyhow!("ml-kem: invalid public key"))?;
    let (ct, ss) = kem.encapsulate(pk_ref)?;
    Ok((Ciphertext(ct.into_vec()), SharedSecret(ss.into_vec())))
}

#[cfg(not(feature = "pq-crypto"))]
pub fn encapsulate(_pk: &PublicKey) -> Result<(Ciphertext, SharedSecret)> {
    Ok((Ciphertext(Vec::new()), SharedSecret(Vec::new())))
}

#[cfg(feature = "pq-crypto")]
pub fn decapsulate(sk: &SecretKey, ct: &Ciphertext) -> Result<SharedSecret> {
    let kem = Kem::new(Algorithm::MlKem768)?;
    let sk_ref = kem
        .secret_key_from_bytes(&sk.0)
        .ok_or_else(|| anyhow::anyhow!("ml-kem: invalid secret key"))?;
    let ct_ref = kem
        .ciphertext_from_bytes(&ct.0)
        .ok_or_else(|| anyhow::anyhow!("ml-kem: invalid ciphertext"))?;
    let ss = kem.decapsulate(sk_ref, ct_ref)?;
    Ok(SharedSecret(ss.into_vec()))
}

#[cfg(not(feature = "pq-crypto"))]
pub fn decapsulate(_sk: &SecretKey, _ct: &Ciphertext) -> Result<SharedSecret> {
    Ok(SharedSecret(Vec::new()))
}
