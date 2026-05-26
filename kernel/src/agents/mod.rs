//! SKYNET agent stubs. Each agent runs as a task on the shared Tokio runtime
//! and communicates with the rest of the kernel exclusively through the bus.

pub mod skynet_a1;
pub mod skynet_a2;
pub mod skynet_a3;
pub mod skynet_a4;
pub mod skynet_a5;
