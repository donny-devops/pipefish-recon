#![allow(dead_code, unused_imports, unused_variables)]

//! PipeFish RECON Agentic OS — kernel entrypoint.
//!
//! Boots the five runtime contexts and five RECON agents concurrently
//! atop a single Tokio runtime, wires them together through an mpsc bus,
//! loads `POLICY.md` as the deny-by-default tool ACL, and handles graceful
//! shutdown on SIGTERM / Ctrl+C.

use std::sync::Arc;

use anyhow::Result;
use tokio::signal;
use tracing::{info, warn};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use pipefish_recon_kernel::agents::recon_a1::ReconA1;
use pipefish_recon_kernel::agents::recon_a2::ReconA2;
use pipefish_recon_kernel::agents::recon_a3::ReconA3;
use pipefish_recon_kernel::agents::recon_a4::ReconA4;
use pipefish_recon_kernel::agents::recon_a5::ReconA5;
use pipefish_recon_kernel::contexts::{
    bus::BusContext, core::CoreContext, dx::DxContext, llm::LlmContext, tool::ToolContext,
};
use pipefish_recon_kernel::mcp::policy::ToolPolicy;
use pipefish_recon_kernel::operator::DEFAULT_BIND;

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

    println!("PipeFish RECON Agentic OS v0.2.0 — kernel online");
    info!("kernel boot sequence initiated");

    let policy = Arc::new(ToolPolicy::load()?);
    info!(rules = policy.allow_count(), "POLICY.md ACL loaded");

    let bind = std::env::var("PIPEFISH_OPERATOR_BIND").unwrap_or_else(|_| DEFAULT_BIND.to_string());

    let (bus, bus_tx, dx_rx) = BusContext::new();
    let core = CoreContext::new(bus_tx.clone(), policy.clone());
    let llm = LlmContext::new(bus_tx.clone());
    let tool = ToolContext::new(bus_tx.clone(), policy.clone());
    let dx = DxContext::new(bus_tx.clone(), dx_rx, policy, bind);
    let a1 = ReconA1::new(bus_tx.clone());
    let a2 = ReconA2::new(bus_tx.clone());
    let a3 = ReconA3::new(bus_tx.clone());
    let a4 = ReconA4::new(bus_tx.clone());
    let a5 = ReconA5::new(bus_tx);

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
            let (
                core_r, bus_r, llm_r, tool_r, dx_r,
                a1_r, a2_r, a3_r, a4_r, a5_r,
            ) = tokio::join!(
                core.run(),
                bus.run(),
                llm.run(),
                tool.run(),
                dx.run(),
                a1.run(),
                a2.run(),
                a3.run(),
                a4.run(),
                a5.run(),
            );
            core_r?; bus_r?; llm_r?; tool_r?; dx_r?;
            a1_r?; a2_r?; a3_r?; a4_r?; a5_r?;
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
