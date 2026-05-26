//! Five domain-scoped runtime contexts that compose the Agentic OS kernel.
//!
//! Each context is privilege-isolated and communicates exclusively through
//! the mpsc bus established by [`bus::BusContext`].

pub mod bus;
pub mod core;
pub mod dx;
pub mod llm;
pub mod tool;
