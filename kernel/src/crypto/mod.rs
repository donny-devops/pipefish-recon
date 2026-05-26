//! Post-quantum cryptographic primitives.
//!
//! - [`ml_kem`] — FIPS 203 ML-KEM-768 key encapsulation
//! - [`slh_dsa`] — FIPS 205 SLH-DSA stateless hash-based signatures
//!
//! Both modules compile to stub types unless the `pq-crypto` Cargo feature
//! is enabled, which pulls in `oqs` (Open Quantum Safe) bindings.

pub mod envelope;
pub mod ml_kem;
pub mod slh_dsa;
