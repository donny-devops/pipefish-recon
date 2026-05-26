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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_returns_ok() {
        let kp = MlKemKeyPair::generate();
        assert!(kp.is_ok());
    }

    #[cfg(not(feature = "pq-crypto"))]
    #[test]
    fn stub_generate_returns_empty_keys() {
        let kp = MlKemKeyPair::generate().unwrap();
        assert!(kp.public.0.is_empty());
        assert!(kp.secret.0.is_empty());
    }

    #[cfg(not(feature = "pq-crypto"))]
    #[test]
    fn stub_encapsulate_returns_empty_ct_and_ss() {
        let kp = MlKemKeyPair::generate().unwrap();
        let (ct, ss) = encapsulate(&kp.public).unwrap();
        assert!(ct.0.is_empty());
        assert!(ss.0.is_empty());
    }

    #[cfg(not(feature = "pq-crypto"))]
    #[test]
    fn stub_decapsulate_returns_empty_ss() {
        let kp = MlKemKeyPair::generate().unwrap();
        let (ct, _) = encapsulate(&kp.public).unwrap();
        let ss = decapsulate(&kp.secret, &ct).unwrap();
        assert!(ss.0.is_empty());
    }

    #[test]
    fn public_key_clones() {
        let pk = PublicKey(vec![1, 2, 3]);
        let pk2 = pk.clone();
        assert_eq!(pk.0, pk2.0);
    }

    #[test]
    fn secret_key_clones() {
        let sk = SecretKey(vec![4, 5, 6]);
        let sk2 = sk.clone();
        assert_eq!(sk.0, sk2.0);
    }

    #[test]
    fn ciphertext_clones() {
        let ct = Ciphertext(vec![7, 8]);
        let ct2 = ct.clone();
        assert_eq!(ct.0, ct2.0);
    }

    #[test]
    fn shared_secret_clones() {
        let ss = SharedSecret(vec![9]);
        let ss2 = ss.clone();
        assert_eq!(ss.0, ss2.0);
    }
}
