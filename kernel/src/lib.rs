#![allow(dead_code, unused_imports, unused_variables)]

//! PipeFish RECON Agentic OS — kernel library.
//!
//! Provides the core context abstractions, post-quantum cryptographic
//! primitives (ML-KEM-768, SLH-DSA), MCP server registry and policy engine,
//! Conventional Commits engine, agent lifecycle management, and the
//! localhost operator HTTP API.

pub mod agents;
pub mod commits;
pub mod contexts;
pub mod crypto;
pub mod events;
pub mod mcp;
pub mod operator;
