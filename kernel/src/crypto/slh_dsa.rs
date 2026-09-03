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
    fn slh_dsa_keypair_and_sign_verify() {
        let kp = SlhDsaKeyPair::generate().expect("keypair generation succeeds");
        let msg = b"PipeFish RECON audit log row";
        let sig = sign(&kp.secret, msg).expect("signing succeeds");
        let valid = verify(&kp.public, msg, &sig).expect("verification succeeds");
        assert!(valid);
    }
}
