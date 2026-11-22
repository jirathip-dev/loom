#![forbid(unsafe_code)]
#![allow(
    clippy::missing_errors_doc,
    reason = "one doc line a public item; each error type names its failures"
)]

mod canonical;
mod ids;
mod log;
mod registry;
mod schedule;
mod streams;

pub use canonical::{CanonicalError, canonical};
pub use ids::{NextId, Seed, SimId, StateHash, Tick};
pub use log::{FORMAT, InputLog, LogHeader, MAGIC, Refused, SaveHeader, Stamped};
pub use registry::{HashReport, Registry, RegistryError, SectionName, State};
pub use schedule::{Runner, in_order};
pub use streams::{Stream, StreamName, Streams};
