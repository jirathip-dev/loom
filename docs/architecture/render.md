# loom-render

The GPU through `wgpu`, game-blind. It knows meshes, materials, a sun, an eye and a list of draws; it knows no
terrain, no simulation and no window library. It is the one crate that may hold `unsafe` (R5,
`crates.md`), and is expected to need none.

## Shapes

```rust
impl Gpu {
    pub fn on_window(window: impl Into<SurfaceTarget<'static>>, size: (u32, u32)) -> Result<Self, GpuError>;
    pub fn offscreen(size: (u32, u32)) -> Result<Self, GpuError>;
    pub fn resize(&mut self, size: (u32, u32));
    pub fn mesh(&mut self, vertices: &[Vertex], indices: &[u32]) -> MeshId;
    pub fn free(&mut self, mesh: MeshId);
    pub fn texture(&mut self, pixels: &[u8], size: (u32, u32)) -> TextureId;
    pub fn material(&mut self, material: &Material) -> MaterialId;
    pub fn render(&mut self, scene: &Scene) -> Result<FrameStats, GpuError>;
    pub fn pixels(&mut self) -> Result<Vec<u8>, GpuError>;
}

pub struct Vertex { pub at: [f32; 3], pub normal: [f32; 3], pub uv: [f32; 2], pub colour: [u8; 4] }
pub struct Material { pub colour: [f32; 3], pub sheen: [f32; 3], pub roughness: f32, pub fuzz: f32, pub fibre: TextureId }
pub struct Instance { pub place: Mat4, pub tint: [f32; 4] }
pub struct Eye { pub view: Mat4, pub proj: Mat4 }

pub struct Scene { /* eye, light, sky, ambient, the draws */ }
impl Scene {
    pub fn new(eye: Eye, light: Light, sky: [f32; 3], ambient: [f32; 3]) -> Self;
    pub fn draw(&mut self, mesh: MeshId, material: MaterialId, at: &[Instance]);
}
pub struct Light { pub towards: Vec3, pub colour: [f32; 3] }
pub struct FrameStats { pub cpu_ms: f32, pub gpu_ms: f32, pub draws: u32, pub triangles: u32 }
```

One vertex layout and one material model for everything. The passes are fixed, one Rust file and one WGSL
file each: three shadow cascades, a depth and normal prepass, ambient occlusion, the lit pass (physically
based, with a cloth sheen term, the fibre texture and a normal map), the sky, then tone mapping with FXAA.

## Usage

```rust
let mesh: MeshId = gpu.mesh(&vertices, &indices);
let cloth: MaterialId = gpu.material(&Material { colour, sheen, roughness, fuzz, fibre });

let mut scene = Scene::new(eye, Light { towards, colour: light }, sky, ambient);
scene.draw(mesh, cloth, &[Instance { place, tint }]);
gpu.render(&scene)?;
```

`Gpu::offscreen` and `pixels` render to an image with no window, for tests and for a caller's screenshots.

Built so far: `on_window`, `offscreen`, `resize`, `mesh`, `free`, `texture`, `material`, `render` and
`pixels`; the lit pass, the sky and tone mapping with FXAA. A fibre texture's rgb is a tangent-space normal and
its alpha the fibre's shade. On a window, `render` presents the frame and skips it while the surface is lost
or covered; `pixels` reads offscreen images only. Planned: the shadow cascades, the prepass and ambient
occlusion.

## Tests

The tests assert relations between pixels (the sunward side is brighter; two renders of one scene are
byte-identical), never a kept image, so they hold on any GPU. With `LOOM_GPU=required` a test fails when no
GPU adapter is found; CI sets it on the hosted macOS leg and the Windows job. Without it, as on the Linux leg,
the GPU tests report themselves skipped, and every shader still validates there.

Target: 60 frames a second at 1920x1080 on an Apple Silicon Mac and on a mid-range desktop GPU
(`docs/decisions/001-foundation.md`).
