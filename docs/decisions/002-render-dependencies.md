# 002: the renderer's dependencies

Status: accepted, 2026-11-24.

`loom-render` is the `wgpu` renderer that `001-foundation.md` plans. This record pins what it links.

| Crate | Used by | Why | Pinned |
|---|---|---|---|
| `wgpu` | loom-render | the GPU on Metal, DX12 and Vulkan; its `naga` validates every WGSL file in a test that needs no GPU | 30.0.2 |
| `bytemuck` | loom-render | vertices, instances and uniforms as bytes, by derive | 1.25.2 |

- `wgpu` is built with default features off and `std`, `dx12`, `metal`, `vulkan` and `wgsl` on: no GL and no
  browser backend. The owner approved the 61 packages it adds to `Cargo.lock` (#6).
- `glam` gains its `bytemuck` feature for `Mat4` in an instance; it stays on `libm` (R6).
- `deny.toml` allows the `ISC` licence (`libloading`, which loads the Vulkan library) and skips
  `rustc-hash` 1.1.0, which `naga` 30 uses while `bevy_ecs` reaches 2.
- `loom-render` is the one crate named in gatekeeper's `[rust] unsafe_crates`: its root says
  `#![deny(unsafe_code)]`, its own `[lints]` deny `clippy::undocumented_unsafe_blocks`, and `rust/bans` fails an
  `unsafe` block without a `// SAFETY:` line. It holds no `unsafe` block today.
- The GPU tests run where `LOOM_GPU=required` is set (the hosted macOS leg and the Windows job) and fail
  there without an adapter; elsewhere they report themselves skipped.
