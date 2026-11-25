# 003: the app's dependencies

Status: accepted, 2026-11-25.

`loom-app` is the window, the pacer and the action layer that `docs/architecture/app.md` plans. This record
pins what it links.

| Crate | Used by | Why | Pinned |
|---|---|---|---|
| `winit` | loom-app | the window and its event loop, keys, mouse buttons, the wheel and the cursor | 0.30.13 |
| `gilrs` | loom-app, macOS and Windows only | gamepad buttons and sticks | 0.11.2 |

- `winit` is the latest stable release, built with default features off and `rwh_06` and `x11` on. On Linux
  it opens X11 only, and loads Xlib and xkbcommon at run time, so the Linux leg builds with no system package;
  a Wayland desktop runs it through XWayland. `0.31` was in beta.
- `gilrs` is a dependency on macOS and Windows only: on Linux it needs `libudev`, a system package. There the
  action layer builds with no gamepad behind it.
- The owner approved the 103 packages they add to `Cargo.lock` (#7), 173 to 276.
- `deny.toml` skips 13 duplicate versions `winit` 0.30 brings: its `objc2` 0.5 family next to `wgpu`'s 0.6 on
  macOS, `windows-sys`, `rustix` and `linux-raw-sys` under `calloop`, `bitflags` 1, `thiserror` 1, and the
  Android-only `jni-sys` and Redox-only `redox_syscall` copies. Moving to `winit` 0.31 drops most of them.
- `loom-render` gains `Gpu::on_window` and `resize`: a surface from `wgpu`, its frames presented in display
  refresh order (FIFO). `loom-render` still links no window library; `loom-app` hands it the window.
