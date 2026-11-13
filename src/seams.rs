//! Host seams: every Minecraft client / Fabric / MixinExtras type the port
//! touches sits here, as a thin no-op-default surface the engine binds.
//! Signatures mirror the upstream Java calls one to one.

use std::sync::{Arc, Mutex, OnceLock};

// ---------------------------------------------------------------- logging

pub struct Logger;

impl Logger {
    pub fn new() -> Logger {
        Logger
    }

    pub fn warn(&self, msg: &str) {
        eprintln!("[nicer-skies] WARN: {msg}");
    }
}

// java: FabricLoader.getInstance().getConfigDir().toFile()
pub fn fabric_config_dir() -> std::path::PathBuf {
    std::env::var("HORIZON_CONFIG_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
}

// java: Minecraft.getInstance().level (Option<ClientLevel>)
pub fn minecraft_level() -> Option<u64> {
    HOOKS.get().and_then(|h| h.current_level.lock().unwrap().clone())
}

// java: ((ClientLevelAccessor) level).nicerSkies_getHashedSeed()
pub fn client_level_hashed_seed(level: &u64) -> i64 {
    if let Some(hooks) = HOOKS.get() {
        return (hooks.client_level_hashed_seed_fn.lock().unwrap())(level);
    }
    0
}

// --------------------------------------------------------------- Mth / ARGB

// java: net.minecraft.util.Mth.invSqrt
pub fn mth_inv_sqrt(x: f32) -> f32 {
    1.0 / x.sqrt()
}

pub fn mth_clamp_f32(value: f32, min: f32, max: f32) -> f32 {
    value.clamp(min, max)
}

pub fn mth_clamp_f64(value: f64, min: f64, max: f64) -> f64 {
    value.clamp(min, max)
}

pub fn mth_clamp_i32(value: i32, min: i32, max: i32) -> i32 {
    value.clamp(min, max)
}

// java: net.minecraft.util.ARGB.color(alpha, r, g, b) -> ABGR packed int.
pub fn argb_color(a: i32, r: i32, g: i32, b: i32) -> i32 {
    (a << 24) | (b << 16) | (g << 8) | r
}

// ---------------------------------------------------------- random / noise

pub struct RandomSource {
    seed: i64,
    counter: Mutex<u64>,
}

impl RandomSource {
    // java: RandomSource.create(seed)
    pub fn create(seed: i64) -> RandomSource {
        RandomSource { seed, counter: Mutex::new(0) }
    }

    // java: RandomSource.fork() -> independent source seeded from this one
    pub fn fork(&self) -> RandomSource {
        let c = *self.counter.lock().unwrap();
        RandomSource::create(self.seed ^ ((c as u64).wrapping_mul(0x9E3779B97F4A7C15) as i64))
    }

    // java: nextFloat()
    pub fn next_float(&self) -> f32 {
        let mut c = self.counter.lock().unwrap();
        *c += 1;
        // xorshift on (seed, counter), exact 32-bit float cast semantics
        let mut x = (self.seed as u64).wrapping_mul(0x9E3779B97F4A7C15) ^ *c;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        (x >> 40) as f32 / 8_388_608.0 / 256.0
    }

    /// Engine snapshot used to construct ImprovedNoise from the same state.
    pub fn snapshot(&self) -> i64 {
        self.seed ^ (*self.counter.lock().unwrap() as i64)
    }
}

pub struct PerlinNoise {
    seed: i64,
    octaves: Vec<i32>,
}

impl PerlinNoise {
    // java: PerlinNoise.create(randomSource.fork(), IntStream.of(1..7))
    pub fn create(source: RandomSource, octaves: impl IntoIterator<Item = i32>) -> PerlinNoise {
        PerlinNoise { seed: source.snapshot(), octaves: octaves.into_iter().collect() }
    }

    // java: noise.getValue(x, y, z)
    pub fn get_value(&self, x: f64, y: f64, z: f64) -> f64 {
        perlin_fbm(self.seed, self.octaves.len(), x, y, z)
    }

    // java: noise.getOctaveNoise(0)
    pub fn get_octave_noise(&self, octave: usize) -> PerlinOctave {
        PerlinOctave { seed: self.seed ^ (octave as i64) }
    }
}

pub struct PerlinOctave {
    seed: i64,
}

impl PerlinOctave {
    // java: noiseWithDerivative(x, y, z, out[3])
    pub fn noise_with_derivative(&self, x: f64, y: f64, z: f64, ds: &mut [f64; 3]) {
        let h = 1e-3;
        let v = perlin_single(self.seed, x, y, z);
        ds[0] = (perlin_single(self.seed, x + h, y, z) - v) / h;
        ds[1] = (perlin_single(self.seed, x, y + h, z) - v) / h;
        ds[2] = (perlin_single(self.seed, x, y, z + h) - v) / h;
    }
}

pub struct ImprovedNoise {
    seed: i64,
}

impl ImprovedNoise {
    // java: new ImprovedNoise(randomSource)
    pub fn new(snapshot: &i64) -> ImprovedNoise {
        ImprovedNoise { seed: *snapshot }
    }

    // java: noise.noise(x, y, z)
    pub fn noise(&self, x: f64, y: f64, z: f64) -> f64 {
        perlin_single(self.seed, x, y, z)
    }
}

// Ken Perlin improved-noise reference math, seed-mixed.
fn perlin_single(seed: i64, x: f64, y: f64, z: f64) -> f64 {
    let xs = x.floor() as i64;
    let ys = y.floor() as i64;
    let zs = z.floor() as i64;
    let xf = x - xs as f64;
    let yf = y - ys as f64;
    let zf = z - zs as f64;

    let u = fade(xf);
    let v = fade(yf);
    let w = fade(zf);

    let h = |i: i64, j: i64, k: i64| -> f64 {
        let mix = (seed as u64)
            ^ (i as u64).wrapping_mul(0x9E3779B97F4A7C15)
            ^ (j as u64).wrapping_mul(0xC2B2AE3D27D4EB4F)
            ^ (k as u64).wrapping_mul(0x165667B19E3779F9);
        (mix >> 11) as f64 / 1_048_576.0 - 1.0
    };

    let lerp = |a: f64, b: f64, t: f64| a + t * (b - a);

    lerp(
        lerp(lerp(h(xs, ys, zs), h(xs + 1, ys, zs), u), lerp(h(xs, ys + 1, zs), h(xs + 1, ys + 1, zs), u), v),
        lerp(lerp(h(xs, ys, zs + 1), h(xs + 1, ys, zs + 1), u), lerp(h(xs, ys + 1, zs + 1), h(xs + 1, ys + 1, zs + 1), u), v),
        w,
    )
}

fn perlin_fbm(seed: i64, octaves: usize, x: f64, y: f64, z: f64) -> f64 {
    let mut total = 0.0;
    let mut amp = 1.0;
    let mut freq = 1.0;
    for o in 0..octaves {
        total += perlin_single(seed ^ (o as i64), x * freq, y * freq, z * freq) * amp;
        amp *= 0.5;
        freq *= 2.0;
    }
    total
}

fn fade(t: f64) -> f64 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

// --------------------------------------------------------------- pipelines

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderPipelineRef(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendFactor {
    SrcAlpha,
    One,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendOp {
    ReverseSubtract,
    Add,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlendFunction {
    pub src_factor: BlendFactor,
    pub dst_factor: BlendFactor,
    pub op: BlendOp,
}

// java: BlendFunction.OVERLAY
pub fn blend_overlay() -> BlendFunction {
    BlendFunction { src_factor: BlendFactor::SrcAlpha, dst_factor: BlendFactor::One, op: BlendOp::Add }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VertexBinding {
    PositionColor,
    PositionTex,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindGroupLayout {
    MatricesProjection,
    Sampler0,
}

pub struct RenderPipelineSpec {
    pub location: String,
    pub vertex_shader: String,
    pub fragment_shader: String,
    pub blend: BlendFunction,
    pub vertex_binding: VertexBinding,
    pub bind_group_layouts: Vec<BindGroupLayout>,
}

// java: RenderPipelines.register(builder(...).build())
pub fn register_render_pipeline(spec: RenderPipelineSpec) -> RenderPipelineRef {
    let _ = spec;
    RenderPipelineRef(0)
}

// ------------------------------------------------------------------ buffers

pub const POSITION_COLOR_VERTEX_SIZE: usize = 20; // 3f pos + 4ub color, padded
pub const POSITION_TEX_VERTEX_SIZE: usize = 24; // 3f pos + 2f uv

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuBufferUsage {
    Vertex,
    CopyDst,
}

impl std::ops::BitOr for GpuBufferUsage {
    type Output = GpuBufferUsage;
    fn bitor(self, _rhs: GpuBufferUsage) -> GpuBufferUsage {
        self
    }
}

#[derive(Clone)]
pub struct GpuBuffer {
    pub size: u64,
}

#[derive(Clone)]
pub struct GpuBufferSlice;

impl GpuBuffer {
    // java: buffer.slice()
    pub fn slice(&self) -> GpuBufferSlice {
        GpuBufferSlice
    }
}

// java: RenderSystem.getDevice().createBuffer(label, usage, size)
pub fn create_gpu_buffer(label: &str, usage: GpuBufferUsage, size: u64) -> GpuBuffer {
    let _ = label;
    let _ = usage;
    GpuBuffer { size }
}

// java: createBuffer(label, USAGE_VERTEX, meshData.vertexBuffer())
pub fn create_gpu_buffer_from_mesh(label: &str, usage: GpuBufferUsage, mesh: &[u8]) -> GpuBuffer {
    create_gpu_buffer(label, usage, mesh.len() as u64)
}

// java: RenderSystem.getDevice().createCommandEncoder().writeToBuffer(slice, data)
pub fn write_to_buffer(buffer: &GpuBuffer, data: &[u8]) {
    let _ = (buffer, data);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimitiveTopology {
    Quads,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VertexFormat {
    PositionColor,
    PositionTex,
}

pub struct AutoStorageIndexBuffer {
    topology: PrimitiveTopology,
}

impl AutoStorageIndexBuffer {
    // java: indexBuffer.getBuffer(count) / indexBuffer.type()
    pub fn get_buffer(&self, count: i32) -> GpuBuffer {
        GpuBuffer { size: count as u64 * 4 }
    }

    pub fn type_(&self) -> PrimitiveTopology {
        self.topology
    }
}

// java: RenderSystem.getSequentialBuffer(PrimitiveTopology.QUADS)
pub fn get_sequential_buffer(topology: PrimitiveTopology) -> AutoStorageIndexBuffer {
    AutoStorageIndexBuffer { topology }
}

// --------------------------------------------------------------- mesh build

pub struct ByteBufferBuilder {
    pub bytes: Vec<u8>,
}

impl ByteBufferBuilder {
    // java: ByteBufferBuilder.exactlySized(size)
    pub fn exactly_sized(size: usize) -> ByteBufferBuilder {
        ByteBufferBuilder { bytes: Vec::with_capacity(size) }
    }
}

pub struct MeshData {
    bytes: Vec<u8>,
    index_count: i32,
}

impl MeshData {
    // java: meshData.drawState().indexCount()
    pub fn draw_state(&self) -> MeshDrawState {
        MeshDrawState { index_count: self.index_count }
    }

    // java: meshData.vertexBuffer()
    pub fn vertex_buffer(&self) -> &[u8] {
        &self.bytes
    }
}

pub struct MeshDrawState {
    pub index_count: i32,
}

pub struct BufferBuilder {
    sink: *mut ByteBufferBuilder,
    topology: PrimitiveTopology,
    format: VertexFormat,
    vertex_count: usize,
    colors: Vec<i32>,
    uvs: Vec<(f32, f32)>,
}

impl BufferBuilder {
    // java: new BufferBuilder(byteBufferBuilder, topology, format)
    pub fn new(sink: &mut ByteBufferBuilder, topology: PrimitiveTopology, format: VertexFormat) -> BufferBuilder {
        BufferBuilder {
            sink,
            topology,
            format,
            vertex_count: 0,
            colors: Vec::new(),
            uvs: Vec::new(),
        }
    }

    // java: addVertex(x, y, z)
    pub fn add_vertex(&mut self, x: f32, y: f32, z: f32) -> &mut Self {
        let bytes = [x.to_le_bytes(), y.to_le_bytes(), z.to_le_bytes()];
        unsafe {
            (*self.sink).bytes.extend_from_slice(&bytes[0]);
            (*self.sink).bytes.extend_from_slice(&bytes[1]);
            (*self.sink).bytes.extend_from_slice(&bytes[2]);
            if self.format == VertexFormat::PositionTex {
                // uv written by set_uv in order
            }
        }
        self.vertex_count += 1;
        self
    }

    // java: setColor(argb)
    pub fn set_color(&mut self, color: i32) -> &mut Self {
        self.colors.push(color);
        self
    }

    // java: setUv(u, v)
    pub fn set_uv(&mut self, u: f32, v: f32) -> &mut Self {
        unsafe {
            (*self.sink).bytes.extend_from_slice(&u.to_le_bytes());
            (*self.sink).bytes.extend_from_slice(&v.to_le_bytes());
        }
        self.uvs.push((u, v));
        self
    }

    // java: buildOrThrow()
    pub fn build_or_throw(&mut self) -> MeshData {
        let index_count = match self.topology {
            PrimitiveTopology::Quads => (self.vertex_count as i32 / 4) * 6,
        };
        let bytes = std::mem::take(unsafe { &mut (*self.sink).bytes });
        MeshData { bytes, index_count }
    }

    // java: build() returns the mesh (try-with-resources in Skybox).
    pub fn build(&mut self) -> MeshData {
        self.build_or_throw()
    }
}

// ----------------------------------------------------------------- textures

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddressMode {
    ClampToEdge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterMode {
    Nearest,
}

pub struct GpuSampler;

#[derive(Clone)]
pub struct GpuTextureView;

// java: RenderSystem.getDevice().createSampler(...)
pub fn create_sampler(
    wrap_u: AddressMode,
    wrap_v: AddressMode,
    min_filter: FilterMode,
    mag_filter: FilterMode,
    mip_level: i32,
    max_lod: Option<f32>,
) -> GpuSampler {
    let _ = (wrap_u, wrap_v, min_filter, mag_filter, mip_level, max_lod);
    GpuSampler
}

#[derive(Clone)]
pub struct NativeImage {
    pub width: i32,
    pub height: i32,
    pub pixels: Arc<Mutex<Vec<i32>>>,
}

impl NativeImage {
    // java: skyNativeTex.setPixelABGR(x, y, abgr)
    pub fn set_pixel_abgr(&self, x: usize, y: usize, abgr: i32) {
        let idx = y * self.width as usize + x;
        self.pixels.lock().unwrap()[idx] = abgr;
    }
}

pub struct DynamicTexture {
    pub image: NativeImage,
}

impl DynamicTexture {
    // java: new DynamicTexture(label, w, h, useMipmaps)
    pub fn new(label: &str, width: i32, height: i32, use_mipmaps: bool) -> DynamicTexture {
        let _ = label;
        DynamicTexture {
            image: NativeImage {
                width,
                height,
                pixels: Arc::new(Mutex::new(vec![0; (width * height) as usize])),
            },
        }
    }

    // java: skyTexture.getPixels()
    pub fn get_pixels(&self) -> NativeImage {
        self.image.clone()
    }

    // java: skyTexture.getTextureView()
    pub fn get_texture_view(&self) -> GpuTextureView {
        GpuTextureView
    }

    // java: skyTexture.upload()
    pub fn upload(&self) {}
}

// -------------------------------------------------------------- executors

pub struct ExecutorService;

impl ExecutorService {
    // java: Executors.newCachedThreadPool()
    pub fn new_cached_thread_pool() -> ExecutorService {
        ExecutorService
    }

    // java: skyExecutor.execute(task): run inline on the worker thread here,
    // the CountDownLatch still counts each task exactly once.
    pub fn execute<F: FnOnce()>(&self, task: F) {
        task();
    }
}

#[derive(Clone)]
pub struct CountDownLatch {
    count: Arc<Mutex<i64>>,
}

impl CountDownLatch {
    // java: new CountDownLatch(n)
    pub fn new(n: usize) -> CountDownLatch {
        CountDownLatch { count: Arc::new(Mutex::new(n as i64)) }
    }

    // java: latch.countDown()
    pub fn count_down(&self) {
        let mut c = self.count.lock().unwrap();
        *c -= 1;
    }

    // java: latch.await(5, TimeUnit.SECONDS)
    pub fn await_timeout_ms(&self, ms: u64) -> bool {
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(ms);
        while std::time::Instant::now() < deadline {
            if *self.count.lock().unwrap() <= 0 {
                return true;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        *self.count.lock().unwrap() <= 0
    }
}

// ------------------------------------------------------------------ render

#[derive(Clone, Copy)]
pub struct PoseStack;

impl PoseStack {
    pub fn last(&self) -> PoseStackEntry {
        PoseStackEntry
    }

    // java: pushPose / popPose
    pub fn push_pose(&mut self) {}
    pub fn pop_pose(&mut self) {}

    // java: mulPose(quaternion)
    pub fn mul_pose(&mut self, _q: Matrix4f) {}
}

pub struct PoseStackEntry;

impl PoseStackEntry {
    // java: poseStack.last().pose()
    pub fn pose(&self) -> Matrix4f {
        Matrix4f::identity()
    }
}

#[derive(Clone, Copy, Default)]
pub struct Matrix4f;

impl Matrix4f {
    pub fn identity() -> Matrix4f {
        Matrix4f
    }

    // java: matrix.mul(other)
    pub fn mul(&self, _other: Matrix4f) -> Matrix4f {
        Matrix4f
    }
}

pub struct Matrix4fStack;

impl Matrix4fStack {
    pub fn push_matrix(&mut self) {}
    pub fn pop_matrix(&mut self) {}
    pub fn mul(&mut self, _m: Matrix4f) {}
}

#[derive(Clone, Copy, Default)]
pub struct Vector3f;

impl Vector3f {
    pub fn zero() -> Vector3f {
        Vector3f
    }
}

pub struct RenderTarget;

impl RenderTarget {
    // java: renderTarget.getColorTextureView()
    pub fn get_color_texture_view(&self) -> GpuTextureView {
        GpuTextureView
    }

    // java: renderTarget.getDepthTextureView()
    pub fn get_depth_texture_view(&self) -> GpuTextureView {
        GpuTextureView
    }
}

pub struct MatrixView;

// java: RenderSystem.getModelViewStack()
pub fn get_model_view_stack() -> MatrixView {
    MatrixView
}

impl MatrixView {
    pub fn push_matrix(&mut self) {}
    pub fn mul(&mut self, _m: Matrix4f) {}
    pub fn pop_matrix(&mut self) {}
    /// java: RenderSystem.getModelViewMatrixCopy()
    pub fn copy_matrix(&self) -> Matrix4f {
        Matrix4f::identity()
    }
}

pub struct DynamicUniforms;

impl DynamicUniforms {
    // java: RenderSystem.getDynamicUniforms().writeTransform(matrix, vec4, vec3, matrix)
    pub fn write_transform(&mut self, _m: &Matrix4f, _tint: [f32; 4], _offset: Vector3f, _texel: Matrix4f) -> GpuBufferSlice {
        GpuBufferSlice
    }
}

// java: RenderSystem.getDynamicUniforms()
pub fn get_dynamic_uniforms() -> DynamicUniforms {
    DynamicUniforms
}

pub struct RenderPass;

impl RenderPass {
    pub fn set_pipeline(&mut self, _pipeline: RenderPipelineRef) {}
    pub fn set_uniform(&mut self, _name: &str, _slice: &GpuBufferSlice) {}
    pub fn bind_texture(&mut self, _name: &str, _view: &GpuTextureView, _sampler: &GpuSampler) {}
    pub fn set_vertex_buffer(&mut self, _index: usize, _slice: &GpuBufferSlice) {}
    pub fn set_index_buffer(&mut self, _slice: &GpuBuffer, _topology: PrimitiveTopology) {}
    pub fn draw_indexed(&mut self, _count: i32, _instances: i32, _first: i32, _base: i32, _start: i32) {}
}

// java: encoder.createRenderPass(label, color, Optional.empty(), depth, OptionalDouble.empty())
pub fn create_render_pass(label: &str, color: &GpuTextureView, depth: &GpuTextureView) -> RenderPass {
    let _ = (label, color, depth);
    RenderPass
}

// java: RenderSystem.bindDefaultUniforms(renderPass)
pub fn bind_default_uniforms(_render_pass: &mut RenderPass) {}

// java: RenderSystem.getModelViewMatrixCopy()
pub fn get_model_view_matrix_copy() -> Matrix4f {
    Matrix4f::identity()
}

// java: com.mojang.math.Axis.XP.rotation(angle)
pub enum Axis {
    XP,
}

impl Axis {
    pub fn rotation(&self, _angle: f64) -> Matrix4f {
        Matrix4f::identity()
    }
}

// ---------------------------------------------------------------- profiler

pub struct ProfilerFiller;

impl ProfilerFiller {
    // java: Profiler.get()
    pub fn get() -> ProfilerFiller {
        ProfilerFiller
    }

    pub fn push(&mut self, _name: &str) {}
    pub fn pop(&mut self) {}
}

// --------------------------------------------------------------------- gui

#[derive(Clone, Debug)]
pub struct Component(pub String);

impl Component {
    // java: Component.translatable(key)
    pub fn translatable(key: &str) -> Component {
        Component(key.to_string())
    }

    // java: Component.translatable(key, args...)
    pub fn translatable_with(key: &str, args: &[String]) -> Component {
        Component(format!("{}|{}", key, args.join(",")))
    }
}

pub type ScreenHandle = u64;

pub struct Font;

pub fn font() -> Font {
    Font
}

pub struct GuiGraphicsExtractor;

impl GuiGraphicsExtractor {
    // java: graphics.fill(x1, y1, x2, y2, color)
    pub fn fill(&mut self, _x1: i32, _y1: i32, _x2: i32, _y2: i32, _color: i32) {}
}

pub struct Tooltip;

impl Tooltip {
    // java: Tooltip.create(component)
    pub fn create(_component: Component) -> Tooltip {
        Tooltip
    }
}

pub struct CheckboxOnValueChange(pub fn(*const (), bool));
pub struct SliderMovedFn(pub Box<dyn Fn(f64) -> Component>);
pub struct SliderReleasedFn(pub Box<dyn Fn(f64)>);

pub struct Checkbox {
    pub active: bool,
    pub alpha: f32,
    pub message: Component,
    pub tooltip: Option<Tooltip>,
}

impl Checkbox {
    // java: new Checkbox(i, j, 128, component, font, bl, onValueChange)
    pub fn new(_i: i32, _j: i32, _width: i32, message: Component, _font: &Font, _selected: bool, _on_value_change: CheckboxOnValueChange) -> Checkbox {
        Checkbox { active: true, alpha: 1.0, message, tooltip: None }
    }

    // java: setTooltip
    pub fn set_tooltip(&self, tooltip: Option<Tooltip>) {
        let _ = tooltip;
    }

    pub fn set_alpha(&mut self, alpha: f32) {
        self.alpha = alpha;
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn extract_contents(&self, _g: &mut GuiGraphicsExtractor, _i: i32, _j: i32, _f: f32) {}
}

pub struct AbstractSliderButton {
    pub value: f64,
    pub message: Component,
    pub tooltip: Option<Tooltip>,
}

impl AbstractSliderButton {
    // java: new AbstractSliderButton(x, y, w, h, text, value)
    pub fn new(_x: i32, _y: i32, _width: i32, _height: i32, message: Component, value: f64) -> AbstractSliderButton {
        AbstractSliderButton { value, message, tooltip: None }
    }

    pub fn set_tooltip(&self, tooltip: Option<Tooltip>) {
        let _ = tooltip;
    }

    pub fn set_message(&mut self, message: Component) {
        self.message = message;
    }
}

#[derive(Clone)]
pub struct Button {
    pub active: bool,
}

impl Button {
    // java: Button.builder(label, onPress)
    pub fn builder(label: Component, on_press: Box<dyn Fn()>) -> ButtonBuilder {
        let _ = label;
        ButtonBuilder { on_press: Some(on_press), bounds: None, position: None, size: None }
    }

    // java: handle reuse across init/rebuild
    pub fn clone_handle(&self) -> Button {
        self.clone()
    }
}

pub struct ButtonBuilder {
    on_press: Option<Box<dyn Fn()>>,
    bounds: Option<(i32, i32, i32, i32)>,
    position: Option<(i32, i32)>,
    size: Option<(i32, i32)>,
}

impl ButtonBuilder {
    // java: .bounds(x, y, w, h)
    pub fn bounds(mut self, x: i32, y: i32, w: i32, h: i32) -> Self {
        self.bounds = Some((x, y, w, h));
        self
    }

    // java: .pos(x, y)
    pub fn pos(mut self, x: i32, y: i32) -> Self {
        self.position = Some((x, y));
        self
    }

    // java: .size(w, h)
    pub fn size(mut self, w: i32, h: i32) -> Self {
        self.size = Some((w, h));
        self
    }

    // java: .build()
    pub fn build(self) -> Button {
        let _ = self.on_press;
        Button { active: true }
    }
}

pub struct StringWidget {
    pub message: Component,
}

impl StringWidget {
    // java: new StringWidget(component, font)
    pub fn new(message: Component, _font: &Font) -> StringWidget {
        StringWidget { message }
    }

    // java: getWidth()
    pub fn get_width(&self) -> i32 {
        self.message.0.len() as i32 * 6
    }

    // java: setPosition(x, y)
    pub fn set_position(&self, _x: i32, _y: i32) {}
}

pub struct MultiLineTextWidget {
    pub max_width: i32,
}

impl MultiLineTextWidget {
    // java: new MultiLineTextWidget(x, y, component, font).setMaxWidth(w)
    pub fn new(_x: i32, _y: i32, _message: Component, _font: &Font) -> MultiLineTextWidget {
        MultiLineTextWidget { max_width: 0 }
    }

    pub fn set_max_width(mut self, width: i32) -> MultiLineTextWidget {
        self.max_width = width;
        self
    }
}

pub enum Widget {
    Checkbox(Checkbox),
    SliderButton(AbstractSliderButton),
    Button(Button),
    StringWidget(StringWidget),
    MultiLineTextWidget(MultiLineTextWidget),
    Separator(super::gui::widget::separator::Separator),
}

// java: minecraft.gui.setScreen(lastScreen)
pub fn set_screen(_screen: ScreenHandle) {}

// ------------------------------------------------- config-screen callbacks

static NEW_CONFIG: OnceLock<Mutex<crate::config::ConfigData>> = OnceLock::new();
static INVALIDATED: OnceLock<Mutex<bool>> = OnceLock::new();
static LAST_SCREEN: OnceLock<Mutex<ScreenHandle>> = OnceLock::new();

fn new_config() -> &'static Mutex<crate::config::ConfigData> {
    NEW_CONFIG.get_or_init(|| Mutex::new(crate::config::ConfigData::default()))
}

fn invalidated() -> &'static Mutex<bool> {
    INVALIDATED.get_or_init(|| Mutex::new(false))
}

// java: newConfig.setRenderNebulas(selected)
pub fn screen_new_config_set_render_nebulas(v: bool) {
    new_config().lock().unwrap().render_nebulas = v;
}

// java: newConfig.setTwinklingStars(selected)
pub fn screen_new_config_set_twinkling_stars(v: bool) {
    new_config().lock().unwrap().twinkling_stars = v;
}

// java: newConfig.getNebulaConfig().setRenderDuringDay(selected)
pub fn screen_nebula_config_set_render_during_day(v: bool) {
    new_config().lock().unwrap().nebula_config.render_during_day = v;
}

// java: newConfig.getNebulaConfig().setNebulaStrength(value)
pub fn screen_nebula_config_set_nebula_strength(v: f32) {
    new_config().lock().unwrap().nebula_config.nebula_strength = v;
}

// java: newConfig.getNebulaConfig().setNebulaNoiseAmount(value)
pub fn screen_nebula_config_set_nebula_noise_amount(v: f32) {
    new_config().lock().unwrap().nebula_config.nebula_noise_amount = v;
}

// java: newConfig.getNebulaConfig().setNebulaNoiseScale(value)
pub fn screen_nebula_config_set_nebula_noise_scale(v: f32) {
    new_config().lock().unwrap().nebula_config.nebula_noise_scale = v;
}

// java: newConfig.getNebulaConfig().setBaseColourAmount(value)
pub fn screen_nebula_config_set_base_colour_amount(v: i32) {
    new_config().lock().unwrap().nebula_config.base_colour_amount = v;
}

// java: invalidated = true / false
pub fn screen_set_invalidated(v: bool) {
    *invalidated().lock().unwrap() = v;
}

// java: this.rebuildWidgets()
pub fn screen_rebuild_widgets() {}

// java: reset button: newConfig.setNebulaConfig(DEFAULT_CONFIG.getNebulaConfig().toBuilder().build())
pub fn screen_reset_nebula_config_to_default() {
    new_config().lock().unwrap().nebula_config = crate::config::Config::DEFAULT_CONFIG.nebula_config.clone();
}

// java: apply button: config.updateConfig(newConfig)
pub fn screen_apply_config() {
    let data = new_config().lock().unwrap().clone();
    let mut instance = crate::get_instance().lock().unwrap();
    if let Some(inst) = instance.as_mut() {
        inst.config.update_config(data);
    }
}

// java: screen: regenerateSky()
pub fn screen_regenerate_sky() {
    let mut instance = crate::get_instance().lock().unwrap();
    if let Some(inst) = instance.as_mut() {
        if crate::core::hashed_seed_manager::can_generate_sky() {
            let seed = crate::core::hashed_seed_manager::get_seed();
            inst.renderer.generate_sky(seed);
        }
    }
}

// java: screen: onClose()
pub fn screen_on_close() {
    let last = *LAST_SCREEN.get_or_init(|| Mutex::new(0)).lock().unwrap();
    set_screen(last);
}

// java: private regenerateSky wrapper used by ConfigScreen
pub fn config_screen_regenerate() {
    screen_regenerate_sky();
}

// ------------------------------------------------------------ mixin plumbing

pub struct EngineHooks {
    pub current_level: Mutex<Option<u64>>,
    client_level_hashed_seed_fn: Mutex<Box<dyn Fn(&u64) -> i64 + Send>>,
    tick_stars_fn: Mutex<Box<dyn Fn() + Send>>,
    build_stars_fn: Mutex<Box<dyn Fn() + Send>>,
    render_nebulas_fn: Mutex<Box<dyn Fn(&mut PoseStack, f32, f32, &RenderTarget) + Send>>,
    render_stars_fn: Mutex<Box<dyn Fn(f32, &PoseStack, &RenderTarget) -> bool + Send>>,
}

pub static HOOKS: OnceLock<EngineHooks> = OnceLock::new();

// java: ClientLevelMixin registration (client entry).
pub fn register_client_level_hooks(
    save_seed: impl Fn(i64) + Send + 'static,
    tick_stars: fn(),
) {
    let _ = save_seed;
    let hooks = EngineHooks {
        current_level: Mutex::new(None),
        client_level_hashed_seed_fn: Mutex::new(Box::new(|_| 0)),
        tick_stars_fn: Mutex::new(Box::new(tick_stars)),
        build_stars_fn: Mutex::new(Box::new(|| {})),
        render_nebulas_fn: Mutex::new(Box::new(|_, _, _, _| {})),
        render_stars_fn: Mutex::new(Box::new(|_, _, _| false)),
    };
    let _ = HOOKS.set(hooks);
}

// java: SkyRendererMixin registration (client entry).
pub fn register_sky_renderer_hooks(
    build_stars: fn(),
    render_nebulas: impl Fn(&mut PoseStack, f32, f32, &RenderTarget) + Send + 'static,
    render_stars: impl Fn(f32, &PoseStack, &RenderTarget) -> bool + Send + 'static,
) {
    let hooks = HOOKS.get_or_init(|| EngineHooks {
        current_level: Mutex::new(None),
        client_level_hashed_seed_fn: Mutex::new(Box::new(|_| 0)),
        tick_stars_fn: Mutex::new(Box::new(|| {})),
        build_stars_fn: Mutex::new(Box::new(|| {})),
        render_nebulas_fn: Mutex::new(Box::new(|_, _, _, _| {})),
        render_stars_fn: Mutex::new(Box::new(|_, _, _| false)),
    });
    *hooks.build_stars_fn.lock().unwrap() = Box::new(build_stars);
    *hooks.render_nebulas_fn.lock().unwrap() = Box::new(render_nebulas);
    *hooks.render_stars_fn.lock().unwrap() = Box::new(render_stars);
}

/// java: the mixin state lives on the ClientLevel instance; the engine keeps
/// one per level and hands it back here.
pub fn client_level_mixin_state() -> crate::mixin::client_level_mixin::ClientLevelMixinState {
    crate::mixin::client_level_mixin::ClientLevelMixinState::new()
}
