use loom_core::{FORMAT, InputLog, MAGIC, Refused, SaveHeader, StateHash, Tick};
use serde::Deserialize;

#[derive(Debug, PartialEq, Eq, Deserialize)]
struct Setup {
    seed: u64,
}

#[derive(Debug, PartialEq, Eq, Deserialize)]
enum Command {
    Place { at: (u32, u32) },
}

type Log = InputLog<Setup, Command>;

const SIM: u32 = 0;
const HEADER: &str = r#"{"format":1,"sim":0,"setup":{"seed":42}}"#;
const PLACE_10: &str = r#"{"tick":10,"seq":0,"cmd":{"Place":{"at":[16,16]}}}"#;
const PLACE_10_1: &str = r#"{"tick":10,"seq":1,"cmd":{"Place":{"at":[1,2]}}}"#;
const PLACE_500: &str = r#"{"tick":500,"seq":0,"cmd":{"Place":{"at":[3,3]}}}"#;

fn lines(lines: &[&str]) -> String {
    lines.join("\n")
}

#[test]
fn a_sorted_log_parses() -> Result<(), Refused> {
    let log = Log::parse(&lines(&[HEADER, PLACE_10, PLACE_10_1, PLACE_500]), SIM)?;
    assert_eq!(log.header.setup, Setup { seed: 42 });
    let ticks: Vec<(Tick, u32)> = log.entries.iter().map(|e| (e.tick, e.seq)).collect();
    assert_eq!(ticks, [(Tick(10), 0), (Tick(10), 1), (Tick(500), 0)]);
    Ok(())
}

#[test]
fn an_unsorted_log_is_refused() {
    let found = Log::parse(&lines(&[HEADER, PLACE_500, PLACE_10]), SIM);
    assert_eq!(found, Err(Refused::Unsorted { line: 3 }));
    let found = Log::parse(&lines(&[HEADER, PLACE_10_1, PLACE_10]), SIM);
    assert_eq!(found, Err(Refused::Unsorted { line: 3 }));
    let found = Log::parse(&lines(&[HEADER, PLACE_10, PLACE_10]), SIM);
    assert_eq!(found, Err(Refused::Unsorted { line: 3 }));
}

#[test]
fn a_wrong_format_is_refused() {
    let header = r#"{"format":2,"sim":0,"setup":{"shape":"from the future"}}"#;
    let found = Log::parse(&lines(&[header, PLACE_10]), SIM);
    assert_eq!(
        found,
        Err(Refused::Format {
            found: 2,
            expected: FORMAT
        })
    );
}

#[test]
fn a_wrong_sim_is_refused() {
    let found = Log::parse(&lines(&[HEADER, PLACE_10]), 1);
    assert_eq!(
        found,
        Err(Refused::Sim {
            found: 0,
            expected: 1
        })
    );
}

#[test]
fn a_save_header_checks_magic_then_format_then_sim() {
    let header = |magic: &str, format, sim| SaveHeader {
        magic: magic.to_owned(),
        format,
        sim,
        setup: Setup { seed: 1 },
        tick: Tick(0),
        hash: StateHash(0),
    };
    assert_eq!(header(MAGIC, FORMAT, SIM).check(SIM), Ok(()));
    assert_eq!(
        header("png", 9, 9).check(SIM),
        Err(Refused::Magic {
            found: "png".to_owned()
        })
    );
    assert_eq!(
        header(MAGIC, 9, 9).check(SIM),
        Err(Refused::Format {
            found: 9,
            expected: FORMAT
        })
    );
    assert_eq!(
        header(MAGIC, FORMAT, 9).check(SIM),
        Err(Refused::Sim {
            found: 9,
            expected: SIM
        })
    );
}
