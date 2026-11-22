pub fn fs_read() {
    let _ = std::fs::read("state.json");
    let _ = std::fs::read_to_string("state.json");
    let _ = std::fs::write("state.json", "");
    let _ = std::fs::File::open("state.json");
}
