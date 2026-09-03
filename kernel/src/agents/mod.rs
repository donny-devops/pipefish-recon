//! RECON agent stubs. Each agent runs as a task on the shared Tokio runtime
//! and communicates with the rest of the kernel exclusively through the bus.

pub mod recon_a1;
pub mod recon_a2;
pub mod recon_a3;
pub mod recon_a4;
pub mod recon_a5;

pub use recon_a1 as skynet_a1;
pub use recon_a2 as skynet_a2;
pub use recon_a3 as skynet_a3;
pub use recon_a4 as skynet_a4;
pub use recon_a5 as skynet_a5;
