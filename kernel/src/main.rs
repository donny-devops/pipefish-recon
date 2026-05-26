//! SKYNET Agentic OS — kernel entrypoint.
//!
//! Boots the five runtime contexts (core, bus, llm, tool, dx) concurrently
//! atop a single Tokio runtime, wires them together through an mpsc bus, and
//! handles graceful shutdown on SIGTERM / Ctrl+C.

use anyhow::Result;
use tokio::signal;
use tracing::{info, warn};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

mod agents;
mod commits;
mod contexts;
mod crypto;
mod events;
mod mcp;
mod signals;

use agents::skynet_a1::SkynetA1;
use contexts::{
    bus::BusContext, core::CoreContext, dx::DxContext, llm::LlmContext, tool::ToolContext,
};

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_target(true))
        .init();
}

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();

    println!("SKYNET Agentic OS v0.1.0 — kernel online");
    info!("kernel boot sequence initiated");

    let (bus, bus_tx) = BusContext::new();
    let core = CoreContext::new(bus_tx.clone());
    let llm = LlmContext::new(bus_tx.clone());
    let tool = ToolContext::new(bus_tx.clone());
    let dx = DxContext::new(bus_tx.clone());
    let skynet_a1 = SkynetA1::new(bus_tx.clone());

    let shutdown = async {
        tokio::select! {
            _ = signal::ctrl_c() => {
                warn!("received Ctrl+C — initiating graceful shutdown");
            }
            _ = terminate_signal() => {
                warn!("received SIGTERM — initiating graceful shutdown");
            }
        }
    };

    tokio::select! {
        result = async {
            let (core_r, bus_r, llm_r, tool_r, dx_r, a1_r) = tokio::join!(
                core.run(),
                bus.run(),
                llm.run(),
                tool.run(),
                dx.run(),
                skynet_a1.run(),
            );
            core_r?; bus_r?; llm_r?; tool_r?; dx_r?; a1_r?;
            Ok::<(), anyhow::Error>(())
        } => {
            result?;
        }
        _ = shutdown => {
            info!("shutdown signal received; kernel halting");
        }
    }

    Ok(())
}

#[cfg(unix)]
async fn terminate_signal() {
    use tokio::signal::unix::{signal, SignalKind};
    if let Ok(mut sig) = signal(SignalKind::terminate()) {
        sig.recv().await;
    } else {
        std::future::pending::<()>().await;
    }
}

#[cfg(not(unix))]
async fn terminate_signal() {
    std::future::pending::<()>().await;
}
