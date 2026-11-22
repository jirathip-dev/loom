# 001: foundation

Status: accepted, 2026-11-21.

The rules this record sets, each with its enforcer, are in `docs/architecture/`: `determinism.md` and
`crates.md`.

## The stack

- loom is a Rust engine on libraries: `winit` for the window, `wgpu` for the GPU, `bevy_ecs` standalone for
  the world, `glam` for maths, `serde` for every saved or hashed value. It is not built on a full engine.
- The ECS is `bevy_ecs`, default features off, pinned to one version and upgraded only through a decision.
- A dependency over about 50,000 lines needs a decision of its own first.
- No engine feature is built before a game needs it.
- loom never depends on a game: its `deny.toml` allows no git source and no registry but crates.io.

## Determinism

- Same setup and input log give the same state hash, on one platform and one binary. Identity across
  platforms is not promised, and no hash is stored in the repo.
- One PCG stream per system, selected by name, so a new stream never moves another's numbers.
- Entities are visited in `SimId` order. No unordered maps, wall clock, environment, files, threads or std
  transcendentals in simulation code; `glam` is built on `libm`; `bevy_ecs` without its parallel executor.
- One schedule, run on one thread, in the order the caller lists its systems.
- The state hash is XXH3-128 over the canonical bytes of the saved state, per section and in total.
- An input log is JSON lines: commands stamped with a tick and a sequence number. A save is the state plus a
  header. Two version numbers, `format` and `sim`; a mismatch is refused; there are no migrations.

## The renderer (planned)

- Physically based shading with a cloth sheen term; one sun light with cascaded shadow maps; ambient
  occlusion; fine surface detail from a fibre texture and normal maps, never from extra geometry.
- Every mesh, texture and animation is made by code from data. loom has no model importer.
- Target: 60 frames a second at 1920x1080 on an Apple Silicon Mac and on a mid-range desktop GPU.

## Toolchain and dependencies

Rust 1.100 stable, pinned in `mise.toml`, with `cargo-nextest` and `sccache`. Dev profile: `opt-level = 1`
for workspace crates, `3` for dependencies. Exact versions are `Cargo.lock`'s; each direct dependency has its
reason in `.gatekeeper/deps.toml`.

| Crate | Used by | Why | Pinned |
|---|---|---|---|
| `bevy_ecs` | loom-core | the world, queries, the single-threaded schedule; default features off | 0.20.1 |
| `glam` | loom-core | vectors, on `libm` | 0.33.12 |
| `serde` | loom-core | state, logs and headers derive it; the canonical writer is a serializer | 1.0.229 |
| `serde_json` | loom-core, xtask tests | state values, the input log; the graph test reads `cargo metadata` | 1.0.154 |
| `rand_pcg` | loom-core | the streams | 0.10.3 |
| `xxhash-rust` | loom-core | the state hash and the stream selector | 0.8.21 |

`deny.toml` denies two versions of one crate, with two exceptions named there: `hashbrown` 0.16, which
`bevy_platform` pins, and `syn` 2, which `bevy`'s derive macros use. `winit` and `wgpu` are pinned by the
change that creates `loom-app` and `loom-render`.

## The licence

loom is MIT OR Apache-2.0, at the user's option: `LICENSE-MIT` and `LICENSE-APACHE`, "Copyright 2026 the loom
authors". It is not published to crates.io (`publish = false`); the name there belongs to another project.

## The repo

- Gates come from gatekeeper's core and rust packs and are never hand-copied; `ci.yml` only calls gatekeeper.
- CI runs on GitHub's hosted runners only, on every pull request: Linux and macOS through gatekeeper, Windows
  as a plain job (`windows.yml`), and `docs.yml` for links and TOML.
- Issues are welcome; pull requests from outside are not taken yet.
- One file an entry: gates, decisions, lint fixtures and graph-test fixtures. No shared list is hand-edited.
