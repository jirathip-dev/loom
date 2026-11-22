pub fn thread_spawn() {
    let _ = std::thread::spawn(|| {});
    std::thread::scope(|_| {});
    let _ = std::thread::Builder::new();
}
