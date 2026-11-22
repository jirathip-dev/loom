# Determinism

The promise: on one platform and one binary, the same setup and the same input log give the same
`StateHash` at every tick, whether the run goes straight through, is replayed from its log, or is saved and
resumed. loom holds the parts; a game built on it checks the whole promise with its own runs, since loom has
no simulation of its own. Identical results across platforms are not promised.

"Simulation code" below is `loom-core`, `loom-procgen` once it exists, and the caller's own simulation crate.

## The rules and their enforcers

| # | Rule | Enforcer |
|---|---|---|
| D1 | All randomness comes from `Streams`: no ambient generator and no `RandomState` | sim-lints (`disallowed_types`: `std::hash::RandomState`), fixture `ambient-rng`; `loom-core` depends on no generator but `rand_pcg` (`.gatekeeper/deps.toml`, gatekeeper's `core/deps`) |
| D2 | Systems visit entities in `SimId` order, through `in_order` | test `a_step_runs_each_system_once_in_order_then_advances_the_tick` (`crates/loom-core/tests/schedule.rs`) spawns ids out of order and asserts the visit order. A raw query iterator is not linted: `in_order` itself calls one |
| D3 | No `HashMap`, `HashSet` or other unordered collection | sim-lints (`disallowed_types`), fixtures `hash-map`, `hash-set` |
| D4 | Transcendental maths goes through `libm`; `glam` is built on `libm` | sim-lints (`disallowed_methods`: `sin`, `cos`, `tan`, `exp`, `ln`, `powf`, `atan2` and the rest of that family, for `f32` and `f64`), fixture `std-trig`; R6 for `glam` |
| D5 | One schedule, one thread, one total order: the order the caller lists | `Runner::new` chains the systems and sets `bevy_ecs`'s single-threaded executor, held by the same schedule test; R6 keeps the parallel executor out of the build; sim-lints (`std::thread::spawn`, `scope`, `Builder`), fixture `thread-spawn` |
| D6 | No wall clock, environment or file reads | sim-lints (`Instant`, `SystemTime`, `std::env`, `std::fs`), fixtures `wall-clock`, `env-read`, `fs-read`; R10's `std::fs` ban |
| D7 | State never holds an ECS `Entity`, only `SimId` | types: `Registry` takes only `Serialize` types, and `bevy_ecs` is built without its `serialize` feature, so `Entity` cannot be registered |
| D8 | Every component and resource in the world is in the state registry, so it is saved and hashed | `Registry::unregistered(&world)` counts the parts that are neither registered nor loom's own; the caller asserts it is 0 after its runs. No loom test drives it yet |
| D9 | State holds finite floats only | the save path: `serde_json` writes a non-finite float as `null`, which `Registry::restore` cannot read back into a float, so a saved and resumed run fails loudly. No loom test feeds it one yet |

## sim-lints

A gate of this repo (`.gatekeeper/gates/sim-lints.toml`). `ci/sim-lints/run -p <package>...` runs clippy on
the packages named, with `ci/sim-lints/clippy.toml`: the simulation-only `disallowed_types` and
`disallowed_methods` lists, each entry with the rule it serves as its reason. Then it runs the same lints on
`ci/sim-lints/fixture/src/`, one file a banned use, and fails unless every line of every fixture is refused
and every list entry refuses at least one line. It fails if its list is missing.

A caller runs the same runner on its own simulation crate, from the folder cargo keeps loom in, so there is
one list and it lives here. A new banned call is a new list entry and a fixture line it refuses.

## The parts in `loom-core`

| Part | What it guarantees |
|---|---|
| `Streams`, `StreamName` | one PCG stream a name: the run's seed, and the name's XXH3 hash as the stream selector; adding a stream never moves another; a stream saves and resumes where it stopped |
| `in_order` | a query's items sorted by `SimId` |
| `Runner` | every system once a step, in the listed order, on one thread; then `Tick` advances by one |
| `canonical` | the canonical bytes of any `Serialize` value: fixed-width little-endian numbers, floats by their bits, lengths before contents |
| `Registry`, `State`, `HashReport` | the world's state captured by name, entities in `SimId` order; restored into a new world; hashed per section and in total |
| `InputLog`, `Stamped`, `LogHeader`, `SaveHeader` | commands stamped `(tick, seq)`; a log out of order, a wrong `format` or a wrong `sim` version is `Refused` |
