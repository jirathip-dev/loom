# loom

A Rust engine for deterministic, headless-first simulation, built for fast work by people and agents alike.
A game built on loom runs its whole world without a window, gets the same state hash from the same seed and
input log on one platform and binary, and is tested by running it.

Status: early. One crate exists, `loom-core`; the others arrive when a game first needs them.

## Crates

| Crate | What | State |
|---|---|---|
| `loom-core` | ticks, ids, seeded random streams, the canonical state hash, the state registry, the input log and save header, the single-threaded schedule runner | built |
| `loom-procgen` | terrain, chunk meshes, soft-part meshes, a fibre texture and a sky, as plain data from a seed and a recipe | planned |
| `loom-render` | the GPU through `wgpu`: meshes, materials, a scene of draws, fixed passes, to a window or an image | planned |
| `loom-app` | the window and its event loop through `winit`, the tick pacer, input actions | planned |

## Docs

- [How the crates fit together, and their rules](docs/architecture/crates.md)
- [The determinism promise, rule by rule](docs/architecture/determinism.md)
- [`loom-procgen`](docs/architecture/procgen.md), [`loom-render`](docs/architecture/render.md),
  [`loom-app`](docs/architecture/app.md): the planned crates' shapes
- [Decisions](docs/DECISIONS.md), starting with [001, the foundation](docs/decisions/001-foundation.md)
- [Working in this repo](AGENTS.md)

## Build and check

```console
$ cargo test --workspace --locked
$ cargo xtask check          # every gate CI runs, the same command
```

`mise.toml` pins the toolchain: Rust 1.100.

## Contributing

Issues are welcome: bug reports, questions and ideas. Pull requests from outside the project are not taken
yet; open an issue instead.

## Licence

loom is dual-licensed under the [MIT licence](LICENSE-MIT) or the [Apache licence, version 2.0](LICENSE-APACHE),
at your option. You may use, copy and change it under either one; the Apache licence adds an explicit patent
grant. Unless you say otherwise, any contribution you submit for inclusion is licensed under both, as above,
with no further terms.
