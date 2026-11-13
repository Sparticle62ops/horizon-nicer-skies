//! 1:1 port of Skybox (incl. the private PaintTask, CoordMap and
//! TextureLocation inner types and the sky executor thread pool).

use super::skybox_painter::SkyboxPainter;
use crate::seams;
use crate::MOD_ID;
use std::sync::Arc;

pub const RESOLUTION: usize = 768;

pub struct Skybox {
    pub sky_pipeline: seams::RenderPipelineRef,
    sky_texture: seams::DynamicTexture,
    sky_executor: seams::ExecutorService,
    skybox_sampler: seams::GpuSampler,
    pub skybox_buffer: seams::GpuBuffer,
    index_buffer: seams::AutoStorageIndexBuffer,
}

impl Skybox {
    pub fn new() -> Self {
        let sky_pipeline = seams::register_render_pipeline(seams::RenderPipelineSpec {
            // java: withLocation(MOD_ID, "pipeline/twinkling_stars") on both
            // pipelines; upstream reuses the same location string.
            location: format!("{MOD_ID}:pipeline/twinkling_stars"),
            vertex_shader: "core/position_tex".to_string(),
            fragment_shader: "core/position_tex".to_string(),
            blend: seams::blend_overlay(),
            vertex_binding: seams::VertexBinding::PositionTex,
            bind_group_layouts: vec![
                seams::BindGroupLayout::MatricesProjection,
                seams::BindGroupLayout::Sampler0,
            ],
        });

        let skybox_buffer = generate_vertices();

        let skybox_sampler = seams::create_sampler(
            seams::AddressMode::ClampToEdge,
            seams::AddressMode::ClampToEdge,
            seams::FilterMode::Nearest,
            seams::FilterMode::Nearest,
            1,
            None,
        );

        Skybox {
            sky_pipeline,
            sky_texture: seams::DynamicTexture::new("NicerSkies_skybox", (RESOLUTION * 4) as i32, (RESOLUTION * 4) as i32, false),
            sky_executor: seams::ExecutorService::new_cached_thread_pool(),
            skybox_sampler,
            skybox_buffer,
            index_buffer: seams::get_sequential_buffer(seams::PrimitiveTopology::Quads),
        }
    }

    pub fn render(&self, render_target: &seams::RenderTarget, pose_stack: &seams::PoseStack, brightness: f32) {
        // uniform values
        let view_model_matrix = seams::get_model_view_matrix_copy().mul(pose_stack.last().pose());
        let mut alpha = match (crate::get_instance().lock().unwrap()).as_ref() {
            Some(inst) => inst.config.get_nebula_strength(),
            None => 0.0,
        };
        let render_during_day = match (crate::get_instance().lock().unwrap()).as_ref() {
            Some(inst) => inst.config.get_render_during_day(),
            None => false,
        };
        if !render_during_day {
            alpha *= brightness;
        }
        let dynamic_transforms = seams::get_dynamic_uniforms().write_transform(
            &view_model_matrix,
            [alpha, alpha, alpha, alpha],
            seams::Vector3f::zero(),
            seams::Matrix4f::identity(),
        );

        // resources
        let color = render_target.get_color_texture_view();
        let depth = render_target.get_depth_texture_view();

        // render
        {
            let mut render_pass = seams::create_render_pass("Nicer Skies Skybox", &color, &depth);
            render_pass.set_pipeline(self.sky_pipeline);
            seams::bind_default_uniforms(&mut render_pass);
            render_pass.set_uniform("DynamicTransforms", &dynamic_transforms);
            render_pass.bind_texture("Sampler0", &self.sky_texture.get_texture_view(), &self.skybox_sampler);
            render_pass.set_vertex_buffer(0, &self.skybox_buffer.slice());
            render_pass.set_index_buffer(&self.index_buffer.get_buffer(24), self.index_buffer.type_());
            render_pass.draw_indexed(36, 1, 0, 0, 0);
        }
    }

    /// java: public void paint(SkyboxPainter painter).
    pub fn paint(&mut self, painter: &dyn SkyboxPainter) {
        let sky_native_tex = self.sky_texture.get_pixels();
        let latch = seams::CountDownLatch::new(TEX_LOCATION_COUNT);

        for location in TextureLocation::values() {
            let task = PaintTask {
                sky_native_tex: sky_native_tex.clone(),
                painter,
                location,
                latch: latch.clone(),
            };
            self.sky_executor.execute(move || task.run());
        }

        // java: latch.await(5, TimeUnit.SECONDS); failure wraps in
        // RuntimeException("Failed to paint skybox").
        if !latch.await_timeout_ms(5000) {
            panic!("Failed to paint skybox");
        }

        self.sky_texture.upload();
    }
}

const TEX_LOCATION_COUNT: usize = 6;

/// java: private static class PaintTask implements Runnable.
pub struct PaintTask<'a> {
    sky_native_tex: seams::NativeImage,
    painter: &'a dyn SkyboxPainter,
    location: TextureLocation,
    latch: seams::CountDownLatch,
}

impl PaintTask<'_> {
    pub fn run(self) {
        let painter = self.painter;
        for tex_y in 0..RESOLUTION {
            for tex_x in 0..RESOLUTION {
                let x = self.location.x_func(tex_x, tex_y);
                let y = self.location.y_func(tex_x, tex_y);
                let z = self.location.z_func(tex_x, tex_y);

                self.sky_native_tex.set_pixel_abgr(
                    self.location.get_x_location(tex_x),
                    self.location.get_y_location(tex_y),
                    painter.get_texel_colour(x, y, z),
                );
            }
        }

        self.latch.count_down();
    }
}

/// java: private enum CoordMap.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CoordMap {
    X,
    Y,
    One,
    NegOne,
}

impl CoordMap {
    pub fn map(self, tex_x: usize, tex_y: usize) -> f32 {
        match self {
            CoordMap::X => (tex_x as f32 / RESOLUTION as f32) * 2.0 - 1.0,
            CoordMap::Y => (tex_y as f32 / RESOLUTION as f32) * 2.0 - 1.0,
            CoordMap::One => 1.0,
            CoordMap::NegOne => -1.0,
        }
    }
}

/// java: public enum TextureLocation.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TextureLocation {
    Top,
    Bottom,
    PosZ,
    NegZ,
    PosX,
    NegX,
}

pub const TEX_LOCATION_COUNT_RE: usize = TEX_LOCATION_COUNT;

impl TextureLocation {
    pub fn values() -> [TextureLocation; 6] {
        [TextureLocation::Top, TextureLocation::Bottom, TextureLocation::PosZ, TextureLocation::NegZ, TextureLocation::PosX, TextureLocation::NegX]
    }

    pub fn x_func(self, tex_x: usize, tex_y: usize) -> f32 {
        match self {
            TextureLocation::Top | TextureLocation::Bottom | TextureLocation::PosZ | TextureLocation::NegZ => CoordMap::X.map(tex_x, tex_y),
            TextureLocation::PosX => CoordMap::One.map(tex_x, tex_y),
            TextureLocation::NegX => CoordMap::NegOne.map(tex_x, tex_y),
        }
    }

    pub fn y_func(self, tex_x: usize, tex_y: usize) -> f32 {
        match self {
            TextureLocation::Top => CoordMap::One.map(tex_x, tex_y),
            TextureLocation::Bottom => CoordMap::NegOne.map(tex_x, tex_y),
            TextureLocation::PosZ | TextureLocation::NegZ | TextureLocation::PosX | TextureLocation::NegX => CoordMap::Y.map(tex_x, tex_y),
        }
    }

    pub fn z_func(self, tex_x: usize, tex_y: usize) -> f32 {
        match self {
            TextureLocation::Top | TextureLocation::Bottom => CoordMap::Y.map(tex_x, tex_y),
            TextureLocation::PosZ => CoordMap::One.map(tex_x, tex_y),
            TextureLocation::NegZ => CoordMap::NegOne.map(tex_x, tex_y),
            TextureLocation::PosX | TextureLocation::NegX => CoordMap::X.map(tex_x, tex_y),
        }
    }

    pub fn get_x_location(self, tex_x: usize) -> usize {
        tex_x + self.shift_x() * RESOLUTION
    }

    pub fn get_y_location(self, tex_y: usize) -> usize {
        tex_y + self.shift_y() * RESOLUTION
    }

    fn shift_x(self) -> usize {
        match self {
            TextureLocation::Top | TextureLocation::Bottom => 2,
            TextureLocation::PosZ => 1,
            TextureLocation::NegZ => 3,
            TextureLocation::PosX => 2,
            TextureLocation::NegX => 0,
        }
    }

    fn shift_y(self) -> usize {
        match self {
            TextureLocation::Top | TextureLocation::Bottom => 0,
            TextureLocation::PosZ | TextureLocation::NegZ | TextureLocation::PosX | TextureLocation::NegX => 1,
        }
    }
}

/// java: private GpuBuffer generateVertices(): the six cubemap faces, exact
/// vertices and UVs, POSITION_TEX quads.
fn generate_vertices() -> seams::GpuBuffer {
    let mut byte_buffer_builder = seams::ByteBufferBuilder::exactly_sized(24 * seams::POSITION_TEX_VERTEX_SIZE);
    let mut skybox_builder = seams::BufferBuilder::new(&mut byte_buffer_builder, seams::PrimitiveTopology::Quads, seams::VertexFormat::PositionTex);

    // +z face
    skybox_builder.add_vertex(-60.0, -60.0, 60.0).set_uv(0.25, 0.25);
    skybox_builder.add_vertex(-60.0, 60.0, 60.0).set_uv(0.25, 0.5);
    skybox_builder.add_vertex(60.0, 60.0, 60.0).set_uv(0.5, 0.5);
    skybox_builder.add_vertex(60.0, -60.0, 60.0).set_uv(0.5, 0.25);

    // -z face
    skybox_builder.add_vertex(-60.0, -60.0, -60.0).set_uv(0.75, 0.25);
    skybox_builder.add_vertex(60.0, -60.0, -60.0).set_uv(1.0, 0.25);
    skybox_builder.add_vertex(60.0, 60.0, -60.0).set_uv(1.0, 0.5);
    skybox_builder.add_vertex(-60.0, 60.0, -60.0).set_uv(0.75, 0.5);

    // bottom face
    skybox_builder.add_vertex(-60.0, -60.0, -60.0).set_uv(0.5, 0.5);
    skybox_builder.add_vertex(-60.0, -60.0, 60.0).set_uv(0.5, 0.75);
    skybox_builder.add_vertex(60.0, -60.0, 60.0).set_uv(0.75, 0.75);
    skybox_builder.add_vertex(60.0, -60.0, -60.0).set_uv(0.75, 0.5);

    // top face
    skybox_builder.add_vertex(-60.0, 60.0, -60.0).set_uv(0.5, 0.0);
    skybox_builder.add_vertex(60.0, 60.0, -60.0).set_uv(0.75, 0.0);
    skybox_builder.add_vertex(60.0, 60.0, 60.0).set_uv(0.75, 0.25);
    skybox_builder.add_vertex(-60.0, 60.0, 60.0).set_uv(0.5, 0.25);

    // +x face
    skybox_builder.add_vertex(60.0, -60.0, -60.0).set_uv(0.5, 0.25);
    skybox_builder.add_vertex(60.0, -60.0, 60.0).set_uv(0.75, 0.25);
    skybox_builder.add_vertex(60.0, 60.0, 60.0).set_uv(0.75, 0.5);
    skybox_builder.add_vertex(60.0, 60.0, -60.0).set_uv(0.5, 0.5);

    // -x face
    skybox_builder.add_vertex(-60.0, -60.0, -60.0).set_uv(0.0, 0.25);
    skybox_builder.add_vertex(-60.0, 60.0, -60.0).set_uv(0.0, 0.5);
    skybox_builder.add_vertex(-60.0, 60.0, 60.0).set_uv(0.25, 0.5);
    skybox_builder.add_vertex(-60.0, -60.0, 60.0).set_uv(0.25, 0.25);

    let skybox_data = skybox_builder.build();
    seams::create_gpu_buffer_from_mesh("Nicer Skies Skybox", seams::GpuBufferUsage::Vertex, &skybox_data.vertex_buffer())
}
