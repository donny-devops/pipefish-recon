//! Audit sinks for the `dx` context.
//!
//! Every [`crate::events::BusEvent`] that crosses the bus is fanned out to
//! these sinks: a local JSON-lines log on disk (always on) and an optional
//! Google Sheets ledger (enabled by setting the `GOOGLE_SERVICE_ACCOUNT_KEY`
//! and `SKYNET_SHEETS_ID` env vars).

pub mod local_log;
pub mod sheets_client;

pub use local_log::LocalAuditLog;
pub use sheets_client::SheetsClient;
