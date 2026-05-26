//! SLH-DSA (FIPS 205) — stateless hash-based digital signatures (SPHINCS+).
//!
//! Default parameter set: SLH-DSA-SHA2-128s ("small" — slightly larger
//! signatures, faster verification). Feature-gated identically to
//! [`super::ml_kem`].

use anyhow::Result;

#[cfg(feature = "pq-crypto")]
use oqs::sig::{Algorithm, Sig};

#[cfg(feature = "pq-crypto")]
const SLH_DSA_ALG: Algorithm = Algorithm::SphincsShake128sSimple;

#[derive(Debug, Clone)]
pub struct PublicKey(pub Vec<u8>);

#[derive(Debug, Clone)]
pub struct SecretKey(pub Vec<u8>);

#[derive(Debug, Clone)]
pub struct Signature(pub Vec<u8>);

pub struct SlhDsaKeyPair {
    pub public: PublicKey,
    pub secret: SecretKey,
}

#[cfg(feature = "pq-crypto")]
impl SlhDsaKeyPair {
    pub fn generate() -> Result<Self> {
        let sig = Sig::new(SLH_DSA_ALG)?;
        let (pk, sk) = sig.keypair()?;
        Ok(Self {
            public: PublicKey(pk.into_vec()),
            secret: SecretKey(sk.into_vec()),
        })
    }
}

#[cfg(not(feature = "pq-crypto"))]
impl SlhDsaKeyPair {
    pub fn generate() -> Result<Self> {
        Ok(Self {
            public: PublicKey(Vec::new()),
            secret: SecretKey(Vec::new()),
        })
    }
}

#[cfg(feature = "pq-crypto")]
pub fn sign(sk: &SecretKey, msg: &[u8]) -> Result<Signature> {
    let sig = Sig::new(SLH_DSA_ALG)?;
    let sk_ref = sig
        .secret_key_from_bytes(&sk.0)
        .ok_or_else(|| anyhow::anyhow!("slh-dsa: invalid secret key"))?;
    let s = sig.sign(msg, sk_ref)?;
    Ok(Signature(s.into_vec()))
}

#[cfg(not(feature = "pq-crypto"))]
pub fn sign(_sk: &SecretKey, _msg: &[u8]) -> Result<Signature> {
    Ok(Signature(Vec::new()))
}

#[cfg(feature = "pq-crypto")]
pub fn verify(pk: &PublicKey, msg: &[u8], signature: &Signature) -> Result<bool> {
    let sig = Sig::new(SLH_DSA_ALG)?;
    let pk_ref = sig
        .public_key_from_bytes(&pk.0)
        .ok_or_else(|| anyhow::anyhow!("slh-dsa: invalid public key"))?;
    let sig_ref = sig
        .signature_from_bytes(&signature.0)
        .ok_or_else(|| anyhow::anyhow!("slh-dsa: invalid signature"))?;
    Ok(sig.verify(msg, sig_ref, pk_ref).is_ok())
}

#[cfg(not(feature = "pq-crypto"))]
pub fn verify(_pk: &PublicKey, _msg: &[u8], _signature: &Signature) -> Result<bool> {
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_returns_ok() {
        let kp = SlhDsaKeyPair::generate();
        assert!(kp.is_ok());
    }

    #[cfg(not(feature = "pq-crypto"))]
    #[test]
    fn stub_generate_returns_empty_keys() {
        let kp = SlhDsaKeyPair::generate().unwrap();
        assert!(kp.public.0.is_empty());
        assert!(kp.secret.0.is_empty());
    }

    #[cfg(not(feature = "pq-crypto"))]
    #[test]
    fn stub_sign_returns_empty_signature() {
        let kp = SlhDsaKeyPair::generate().unwrap();
        let sig = sign(&kp.secret, b"hello world").unwrap();
        assert!(sig.0.is_empty());
    }

    #[cfg(not(feature = "pq-crypto"))]
    #[test]
    fn stub_verify_always_returns_true() {
        let kp = SlhDsaKeyPair::generate().unwrap();
        let sig = sign(&kp.secret, b"test message").unwrap();
        assert!(verify(&kp.public, b"test message", &sig).unwrap());
    }

    #[cfg(not(feature = "pq-crypto"))]
    #[test]
    fn stub_verify_returns_true_for_wrong_message() {
        // The stub ignores all arguments and always returns true
        let kp = SlhDsaKeyPair::generate().unwrap();
        let sig = sign(&kp.secret, b"original").unwrap();
        assert!(verify(&kp.public, b"different", &sig).unwrap());
    }

    #[test]
    fn public_key_clones() {
        let pk = PublicKey(vec![1, 2]);
        let pk2 = pk.clone();
        assert_eq!(pk.0, pk2.0);
    }

    #[test]
    fn secret_key_clones() {
        let sk = SecretKey(vec![3, 4]);
        let sk2 = sk.clone();
        assert_eq!(sk.0, sk2.0);
    }

    #[test]
    fn signature_clones() {
        let sig = Signature(vec![5, 6]);
        let sig2 = sig.clone();
        assert_eq!(sig.0, sig2.0);
    }
}
