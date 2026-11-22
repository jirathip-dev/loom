use std::fmt;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::{StateHash, Tick};

/// The file encoding version of logs and saves.
pub const FORMAT: u32 = 1;

/// The first field of every save.
pub const MAGIC: &str = "loom-save";

/// A command with the tick it applies at and its order inside that tick.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Stamped<C> {
    pub tick: Tick,
    pub seq: u32,
    pub cmd: C,
}

/// The first line of an input log: its versions and the run's setup.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogHeader<S> {
    pub format: u32,
    pub sim: u32,
    pub setup: S,
}

/// A run's setup and every command it took, sorted by `(tick, seq)`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InputLog<S, C> {
    pub header: LogHeader<S>,
    pub entries: Vec<Stamped<C>>,
}

/// The header of a save: what it is, its versions, its setup, tick and state hash.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveHeader<S> {
    pub magic: String,
    pub format: u32,
    pub sim: u32,
    pub setup: S,
    pub tick: Tick,
    pub hash: StateHash,
}

/// Why a log or a save was refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refused {
    Empty,
    Parse { line: usize, message: String },
    Magic { found: String },
    Format { found: u32, expected: u32 },
    Sim { found: u32, expected: u32 },
    Unsorted { line: usize },
}

impl fmt::Display for Refused {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("the log has no header line"),
            Self::Parse { line, message } => write!(f, "line {line}: {message}"),
            Self::Magic { found } => write!(f, "magic {found:?}, expected {MAGIC:?}"),
            Self::Format { found, expected } => write!(f, "format {found}, expected {expected}"),
            Self::Sim { found, expected } => write!(f, "sim {found}, expected {expected}"),
            Self::Unsorted { line } => {
                write!(f, "line {line}: not after the line before in (tick, seq)")
            }
        }
    }
}

impl std::error::Error for Refused {}

#[derive(Deserialize)]
struct Versions {
    format: u32,
    sim: u32,
}

fn versions(format: u32, sim: u32, expected_sim: u32) -> Result<(), Refused> {
    if format != FORMAT {
        return Err(Refused::Format {
            found: format,
            expected: FORMAT,
        });
    }
    if sim != expected_sim {
        return Err(Refused::Sim {
            found: sim,
            expected: expected_sim,
        });
    }
    Ok(())
}

fn json<T: DeserializeOwned>(line: usize, text: &str) -> Result<T, Refused> {
    serde_json::from_str(text).map_err(|e| Refused::Parse {
        line,
        message: e.to_string(),
    })
}

impl<S: DeserializeOwned, C: DeserializeOwned> InputLog<S, C> {
    /// Reads a JSON-lines log, refusing a wrong `format`, a wrong `sim` or entries out of order.
    pub fn parse(text: &str, sim: u32) -> Result<Self, Refused> {
        let mut lines = text.lines();
        let first = lines.next().ok_or(Refused::Empty)?;
        let found: Versions = json(1, first)?;
        versions(found.format, found.sim, sim)?;
        let header = json(1, first)?;
        let entries = lines
            .enumerate()
            .map(|(i, line)| json::<Stamped<C>>(i + 2, line))
            .collect::<Result<Vec<_>, _>>()?;
        if let Some(i) = entries
            .windows(2)
            .position(|pair| matches!(pair, [a, b] if (a.tick, a.seq) >= (b.tick, b.seq)))
        {
            return Err(Refused::Unsorted { line: i + 3 });
        }
        Ok(Self { header, entries })
    }
}

impl<S> SaveHeader<S> {
    /// Checks `magic`, `format` and `sim`, in that order.
    pub fn check(&self, sim: u32) -> Result<(), Refused> {
        if self.magic != MAGIC {
            return Err(Refused::Magic {
                found: self.magic.clone(),
            });
        }
        versions(self.format, self.sim, sim)
    }
}
