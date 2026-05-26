//! Post-quantum encrypted message envelope for inter-context communication.
//!
//! Wraps a [`BusEvent`] in an ML-KEM-768 (FIPS 203) key-encapsulated,
//! AES-256-GCM-sealed envelope. When the `pq-crypto` feature is disabled the
//! envelope falls back to a plaintext passthrough that exercises the same code
//! path, so the rest of the kernel can call `seal`/`open` unconditionally.

use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::events::{BusEvent, ContextId};

#[cfg(feature = "pq-crypto")]
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
#[cfg(feature = "pq-crypto")]
use hkdf::Hkdf;
#[cfg(feature = "pq-crypto")]
use rand::RngCore;
#[cfg(feature = "pq-crypto")]
use sha2::Sha256;

#[cfg(feature = "pq-crypto")]
use crate::crypto::ml_kem;

/// ML-KEM public key bytes for a recipient context.
#[derive(Debug, Clone)]
pub struct RecipientKey(pub Vec<u8>);

/// ML-KEM private key bytes for a recipient context.
#[derive(Debug, Clone)]
pub struct RecipientPrivKey(pub Vec<u8>);

/// A post-quantum encrypted message envelope for inter-context communication.
/// Uses ML-KEM-768 (FIPS 203) for key encapsulation and AES-256-GCM for payload encryption.
/// When the `pq-crypto` feature is disabled, falls back to plaintext (dev mode).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageEnvelope {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub sender: ContextId,
    pub recipient: ContextId,
    /// ML-KEM ciphertext (encapsulated shared secret) — empty in dev mode
    pub kem_ciphertext: Vec<u8>,
    /// AES-256-GCM nonce — empty in dev mode
    pub nonce: Vec<u8>,
    /// Encrypted (or plaintext in dev mode) serialized BusEvent payload
    pub ciphertext: Vec<u8>,
    /// SLH-DSA signature over (id || timestamp || sender || recipient || ciphertext) — empty stub until pq-crypto wired
    pub signature: Vec<u8>,
}

impl MessageEnvelope {
    #[cfg(feature = "pq-crypto")]
    pub fn seal(
        event: &BusEvent,
        sender: ContextId,
        recipient: ContextId,
        recipient_pub_key: &RecipientKey,
    ) -> Result<Self> {
        let plaintext = serde_json::to_vec(event)?;

        let pk = ml_kem::PublicKey(recipient_pub_key.0.clone());
        let (kem_ct, shared_secret) = ml_kem::encapsulate(&pk)?;

        let hk = Hkdf::<Sha256>::new(None, &shared_secret.0);
        let mut aead_key = [0u8; 32];
        hk.expand(b"skynet-bus-envelope-v1", &mut aead_key)
            .map_err(|_| anyhow!("envelope: HKDF expand failed"))?;

        let cipher = Aes256Gcm::new_from_slice(&aead_key)
            .map_err(|_| anyhow!("envelope: AES-256-GCM key init failed"))?;
        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, plaintext.as_ref())
            .map_err(|_| anyhow!("envelope: AES-256-GCM encrypt failed"))?;

        Ok(Self {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            sender,
            recipient,
            kem_ciphertext: kem_ct.0,
            nonce: nonce_bytes.to_vec(),
            ciphertext,
            signature: Vec::new(),
        })
    }

    #[cfg(not(feature = "pq-crypto"))]
    pub fn seal(
        event: &BusEvent,
        sender: ContextId,
        recipient: ContextId,
        _recipient_pub_key: &RecipientKey,
    ) -> Result<Self> {
        let plaintext = serde_json::to_vec(event)?;
        Ok(Self {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            sender,
            recipient,
            kem_ciphertext: Vec::new(),
            nonce: Vec::new(),
            ciphertext: plaintext,
            signature: Vec::new(),
        })
    }

    #[cfg(feature = "pq-crypto")]
    pub fn open(&self, priv_key: &RecipientPrivKey) -> Result<BusEvent> {
        let sk = ml_kem::SecretKey(priv_key.0.clone());
        let kem_ct = ml_kem::Ciphertext(self.kem_ciphertext.clone());
        let shared_secret = ml_kem::decapsulate(&sk, &kem_ct)?;

        let hk = Hkdf::<Sha256>::new(None, &shared_secret.0);
        let mut aead_key = [0u8; 32];
        hk.expand(b"skynet-bus-envelope-v1", &mut aead_key)
            .map_err(|_| anyhow!("envelope: HKDF expand failed"))?;

        let cipher = Aes256Gcm::new_from_slice(&aead_key)
            .map_err(|_| anyhow!("envelope: AES-256-GCM key init failed"))?;
        if self.nonce.len() != 12 {
            anyhow::bail!("envelope: invalid nonce length {}", self.nonce.len());
        }
        let nonce = Nonce::from_slice(&self.nonce);

        let plaintext = cipher
            .decrypt(nonce, self.ciphertext.as_ref())
            .map_err(|_| anyhow!("envelope: AES-256-GCM decrypt failed"))?;

        let event: BusEvent = serde_json::from_slice(&plaintext)?;
        Ok(event)
    }

    #[cfg(not(feature = "pq-crypto"))]
    pub fn open(&self, _priv_key: &RecipientPrivKey) -> Result<BusEvent> {
        let event: BusEvent = serde_json::from_slice(&self.ciphertext)?;
        Ok(event)
    }
}

/// Holds ML-KEM keypairs for each context. Owned exclusively by CoreContext.
pub struct KeyStore {
    pairs: HashMap<ContextId, (RecipientKey, RecipientPrivKey)>,
}

impl KeyStore {
    pub fn generate() -> Result<Self> {
        let contexts = [
            ContextId::Core,
            ContextId::Bus,
            ContextId::Llm,
            ContextId::Tool,
            ContextId::Dx,
        ];
        let mut pairs = HashMap::with_capacity(contexts.len());
        for ctx in contexts {
            let kp = crate::crypto::ml_kem::MlKemKeyPair::generate()?;
            pairs.insert(
                ctx,
                (RecipientKey(kp.public.0), RecipientPrivKey(kp.secret.0)),
            );
        }
        Ok(Self { pairs })
    }

    pub fn pub_key(&self, ctx: &ContextId) -> Option<&RecipientKey> {
        self.pairs.get(ctx).map(|(pk, _)| pk)
    }

    pub fn priv_key(&self, ctx: &ContextId) -> Option<&RecipientPrivKey> {
        self.pairs.get(ctx).map(|(_, sk)| sk)
    }

    pub fn len(&self) -> usize {
        self.pairs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.pairs.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::{CommitType, ContextScope};

    fn sample_event() -> BusEvent {
        BusEvent::new(
            ContextId::Llm,
            CommitType::Feat,
            ContextScope::Llm,
            "routed threat signal to SKYNET-A3",
        )
    }

    #[test]
    fn seal_then_open_roundtrip() {
        let store = KeyStore::generate().expect("keystore generation");
        let pk = store.pub_key(&ContextId::Tool).expect("pub key");
        let sk = store.priv_key(&ContextId::Tool).expect("priv key");

        let evt = sample_event();
        let envelope = MessageEnvelope::seal(&evt, ContextId::Llm, ContextId::Tool, pk)
            .expect("seal");
        let opened = envelope.open(sk).expect("open");

        assert_eq!(opened.id, evt.id);
        assert_eq!(opened.description, evt.description);
        assert_eq!(opened.commit_type, evt.commit_type);
        assert_eq!(opened.scope, evt.scope);
    }

    #[test]
    fn envelope_serde_roundtrip() {
        let store = KeyStore::generate().expect("keystore generation");
        let pk = store.pub_key(&ContextId::Bus).expect("pub key");

        let envelope =
            MessageEnvelope::seal(&sample_event(), ContextId::Core, ContextId::Bus, pk)
                .expect("seal");

        let json = serde_json::to_string(&envelope).expect("ser");
        let back: MessageEnvelope = serde_json::from_str(&json).expect("de");

        assert_eq!(back.id, envelope.id);
        assert_eq!(back.sender, envelope.sender);
        assert_eq!(back.recipient, envelope.recipient);
        assert_eq!(back.ciphertext, envelope.ciphertext);
        assert_eq!(back.nonce, envelope.nonce);
        assert_eq!(back.kem_ciphertext, envelope.kem_ciphertext);
    }

    #[cfg(feature = "pq-crypto")]
    #[test]
    fn open_with_wrong_key_errors() {
        let store = KeyStore::generate().expect("keystore generation");
        let pk = store.pub_key(&ContextId::Tool).expect("pub key");
        let wrong_sk = store.priv_key(&ContextId::Dx).expect("priv key");

        let envelope =
            MessageEnvelope::seal(&sample_event(), ContextId::Llm, ContextId::Tool, pk)
                .expect("seal");
        assert!(envelope.open(wrong_sk).is_err());
    }
}
