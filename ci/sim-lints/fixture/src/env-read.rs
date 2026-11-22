pub fn env_read() {
    let _ = std::env::var("SEED");
    let _ = std::env::var_os("SEED");
    let _ = std::env::vars();
    let _ = std::env::args();
}
