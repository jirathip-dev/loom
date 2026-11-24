use std::path::Path;
use wgpu::naga;

#[test]
fn every_shader_validates_without_a_gpu() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/shaders");
    let mut checked = Vec::new();
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_none_or(|ext| ext != "wgsl") {
            continue;
        }
        let source = std::fs::read_to_string(&path).unwrap();
        let module = naga::front::wgsl::parse_str(&source)
            .unwrap_or_else(|e| panic!("{}: {}", path.display(), e.emit_to_string(&source)));
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::default(),
        )
        .validate(&module)
        .unwrap_or_else(|e| panic!("{}: {}", path.display(), e.emit_to_string(&source)));
        checked.push(path);
    }
    assert_eq!(checked.len(), 3, "lit, sky and tone: {checked:?}");
}
