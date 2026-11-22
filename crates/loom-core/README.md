# loom-core

The engine's deterministic core: ticks and ids, seeded random streams, the canonical state hash, the state
registry, the input log and save header, and the single-threaded schedule runner. It knows no game and depends
on no other loom crate (R4). Its rules are in `../../docs/architecture/determinism.md`.
