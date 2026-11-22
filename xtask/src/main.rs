#![forbid(unsafe_code)]

use std::path::Path;
use std::process::{Command, ExitCode};

fn main() -> Result<ExitCode, String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or("xtask has no parent folder")?;
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("check") => check(root, args),
        other => Err(format!("unknown verb {other:?}; verbs: check")),
    }
}

fn check(root: &Path, args: impl Iterator<Item = String>) -> Result<ExitCode, String> {
    let status = Command::new(root.join(".gatekeeper/run"))
        .arg("check")
        .args(args)
        .current_dir(root)
        .status()
        .map_err(|e| format!("cannot start .gatekeeper/run: {e}"))?;
    Ok(status
        .code()
        .and_then(|code| u8::try_from(code).ok())
        .map_or(ExitCode::FAILURE, ExitCode::from))
}
