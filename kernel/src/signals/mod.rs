//! Signal ingestion subsystem — schemas and feed pollers used by SKYNET-A1.

pub mod nvd_feed;
pub mod threat_signal;

pub use nvd_feed::NvdFeedPoller;
#[allow(unused_imports)]
pub use threat_signal::{Ioc, IocType, SignalSource, SignalType, ThreatSignal};
