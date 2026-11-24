use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;

const FORBID_UNSAFE: &str = "#![forbid(unsafe_code)]";
const R5_LIFTED: &[(&str, &str)] = &[("loom-render", "#![deny(unsafe_code)]")];

fn r5_root_attribute(name: &str) -> &'static str {
    R5_LIFTED
        .iter()
        .find(|(lifted, _)| *lifted == name)
        .map_or(FORBID_UNSAFE, |(_, attribute)| attribute)
}

struct Graph {
    members: BTreeSet<String>,
    packages: BTreeMap<String, Value>,
    nodes: BTreeMap<String, Value>,
}

impl Graph {
    fn of(dir: &Path) -> Self {
        let out = Command::new(env!("CARGO"))
            .args(["metadata", "--locked", "--format-version", "1"])
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let meta: Value = serde_json::from_slice(&out.stdout).unwrap();
        let by_id = |list: &Value| {
            list.as_array()
                .unwrap()
                .iter()
                .map(|item| (item["id"].as_str().unwrap().to_owned(), item.clone()))
                .collect()
        };
        Self {
            members: meta["workspace_members"]
                .as_array()
                .unwrap()
                .iter()
                .map(|id| id.as_str().unwrap().to_owned())
                .collect(),
            packages: by_id(&meta["packages"]),
            nodes: by_id(&meta["resolve"]["nodes"]),
        }
    }

    fn name(&self, id: &str) -> &str {
        self.packages[id]["name"].as_str().unwrap()
    }

    fn deps(&self, id: &str) -> Vec<String> {
        self.nodes[id]["dependencies"]
            .as_array()
            .unwrap()
            .iter()
            .map(|dep| dep.as_str().unwrap().to_owned())
            .collect()
    }

    fn features(&self, id: &str) -> Vec<&str> {
        self.nodes[id]["features"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| f.as_str().unwrap())
            .collect()
    }

    fn violations(&self) -> Vec<String> {
        let mut found = Vec::new();
        for id in &self.members {
            let name = self.name(id);
            for dep in self.deps(id) {
                if name == "loom-core" && self.members.contains(&dep) {
                    found.push(format!("R4: {name} depends on {}", self.name(&dep)));
                }
            }
            if name.starts_with("loom-") {
                let attribute = r5_root_attribute(name);
                for root in self.crate_roots(id) {
                    let text = std::fs::read_to_string(&root).unwrap();
                    if !text.lines().any(|line| line.trim() == attribute) {
                        found.push(format!("R5: {} lacks {attribute}", root.display()));
                    }
                }
            }
        }
        for id in self.nodes.keys() {
            let features = self.features(id);
            match self.name(id) {
                "bevy_ecs" if features.contains(&"multi_threaded") => {
                    found.push("R6: bevy_ecs is built with multi_threaded".to_owned());
                }
                "glam" if !features.contains(&"libm") => {
                    found.push("R6: glam is built without libm".to_owned());
                }
                _ => {}
            }
        }
        found
    }

    fn crate_roots(&self, id: &str) -> Vec<PathBuf> {
        self.packages[id]["targets"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|target| {
                target["kind"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|kind| ["lib", "bin", "proc-macro"].contains(&kind.as_str().unwrap()))
            })
            .map(|target| PathBuf::from(target["src_path"].as_str().unwrap()))
            .collect()
    }
}

fn fixture(name: &str) -> Vec<String> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/crate-rules");
    Graph::of(&dir.join(name)).violations()
}

fn assert_refused(name: &str, rule: &str) {
    let found = fixture(name);
    assert!(
        !found.is_empty() && found.iter().all(|v| v.starts_with(rule)),
        "{name} should break {rule} alone, found {found:?}"
    );
}

#[test]
fn the_workspace_keeps_the_crate_rules() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    assert_eq!(Graph::of(root).violations(), Vec::<String>::new());
}

#[test]
fn the_r5_lifted_crates_are_gatekeepers_unsafe_crates() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let config = std::fs::read_to_string(root.join(".gatekeeper/config.toml")).unwrap();
    let names: Vec<String> = R5_LIFTED
        .iter()
        .map(|(name, _)| format!("\"{name}\""))
        .collect();
    let line = format!("unsafe_crates = [{}]", names.join(", "));
    assert!(
        config.lines().any(|l| l.trim() == line),
        "config.toml lacks `{line}`"
    );
}

#[test]
fn the_clean_fixture_passes() {
    assert_eq!(fixture("clean"), Vec::<String>::new());
}

#[test]
fn loom_core_depending_on_procgen_is_refused_by_r4() {
    assert_refused("core-uses-procgen", "R4");
}

#[test]
fn a_crate_root_without_forbid_unsafe_is_refused_by_r5() {
    assert_refused("forbid-missing", "R5");
}

#[test]
fn bevy_ecs_with_multi_threaded_is_refused_by_r6() {
    assert_refused("ecs-multi-threaded", "R6");
}

#[test]
fn glam_without_libm_is_refused_by_r6() {
    assert_refused("glam-without-libm", "R6");
}
