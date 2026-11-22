use std::process::Command;

fn xtask(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn an_unknown_verb_is_refused_naming_the_verbs() {
    let out = xtask(&["nope"]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("verbs: check"));
}

#[test]
fn check_passes_the_wrappers_exit_code_on() {
    let out = xtask(&["check", "--gate", "no/such-gate"]);
    assert_eq!(out.status.code(), Some(2));
}
