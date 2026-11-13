//! 1:1 port of Starbox. The STAR_PIPELINE registration, buffer creation,
//! generation, per-tick mesh rebuild and render pass all mirror the java
//! one to one through the engine seams.

use crate::core::gradient::Gradient;
use crate::sky::star::star::Star;
use crate::seams;
use crate::MOD_ID;

pub enum StarboxRef {
    None,
    Some(Box<Starbox>),
}

impl StarboxRef {
    pub fn is_built(&self) -> bool {
        matches!(self, StarboxRef::Some(_))
    }

    pub fn as_star(&self) -> Option<&Starbox> {
        match self {
            StarboxRef::Some(b) => Some(b),
            StarboxRef::None => None,
        }
    }

    pub fn as_mut(&mut self) -> Option<&mut Starbox> {
        match self {
            StarboxRef::Some(b) => Some(b),
            StarboxRef::None => None,
        }
    }
}

pub struct Starbox {
    pub star_pipeline: seams::RenderPipelineRef,
    star_list: Vec<Option<Star>>,
    pub star_buffer: seams::GpuBuffer,
    index_buffer: seams::AutoStorageIndexBuffer,
    pub star_index_count: i32,
}

const STAR_COUNT: usize = 1500;
const STARBOX_SIZE: usize = seams::POSITION_COLOR_VERTEX_SIZE * STAR_COUNT * 4;

impl Starbox {
    pub fn new() -> Self {
        let star_pipeline = seams::register_render_pipeline(seams::RenderPipelineSpec {
            location: format!("{MOD_ID}:pipeline/twinkling_stars"),
            vertex_shader: "core/position_color".to_string(),
            fragment_shader: "core/position_color".to_string(),
            blend: seams::blend_overlay(),
            vertex_binding: seams::VertexBinding::PositionColor,
            bind_group_layouts: vec![seams::BindGroupLayout::MatricesProjection],
        });

        Starbox {
            star_pipeline,
            star_list: (0..STAR_COUNT).map(|_| None).collect(),
            star_buffer: seams::create_gpu_buffer(
                "Star Vertex Buffer",
                seams::GpuBufferUsage::Vertex | seams::GpuBufferUsage::CopyDst,
                STARBOX_SIZE as u64,
            ),
            index_buffer: seams::get_sequential_buffer(seams::PrimitiveTopology::Quads),
            star_index_count: 0,
        }
    }

    pub fn generate_stars(&mut self, random_source: &mut seams::RandomSource, star_gradient: &Gradient) {
        let mut noise = seams::ImprovedNoise::new(&random_source.snapshot());

        // java: for (i = 0..STAR_COUNT) { ... else --i; } -- the else branch
        // re-rolls the same index until the condition accepts, so the Rust
        // mirror is a while loop pushing exactly STAR_COUNT accepted stars.
        let mut stars: Vec<Option<Star>> = Vec::with_capacity(STAR_COUNT);

        while stars.len() < STAR_COUNT {
            let rand_x = random_source.next_float() * 2.0 - 1.0;
            let rand_y = random_source.next_float() * 2.0 - 1.0;
            let rand_z = random_source.next_float() * 2.0 - 1.0;

            let resize_speed = 0.03 + random_source.next_float() * 0.04;
            let spin_speed = random_source.next_float() * 0.02 - 0.01;

            let star_rgb = star_gradient.get_at(random_source.next_float() as f64);
            let star_color = seams::argb_color(star_rgb[0], star_rgb[1], star_rgb[2], 255);

            let star_radius = 0.15 + random_source.next_float() * 0.15;
            let star_value = noise.noise((rand_x * 2.5) as f64, (rand_y * 2.5) as f64, (rand_z * 2.5) as f64) + 0.5;

            let squared_distance = rand_x * rand_x + rand_y * rand_y + rand_z * rand_z;
            if squared_distance < 1.0 && squared_distance > 0.01 && star_value > 0.2 {
                stars.push(Some(Star::new(rand_x, rand_y, rand_z, star_radius, star_color, resize_speed, spin_speed)));
            }
        }

        self.star_list = stars;
    }

pub fn update_stars(&mut self, ticks: i32) {
        let mut profiler_fill = seams::ProfilerFiller::get();

        let mut byte_buffer_builder = seams::ByteBufferBuilder::exactly_sized(STARBOX_SIZE);
        let mut buffer_builder = seams::BufferBuilder::new(&mut byte_buffer_builder, seams::PrimitiveTopology::Quads, seams::VertexFormat::PositionColor);

        profiler_fill.push("tick stars");
        for i in 0..STAR_COUNT {
            self.star_list[i].as_mut().unwrap().tick(ticks);
        }
        profiler_fill.pop();

        profiler_fill.push("set vertices");
        for i in 0..STAR_COUNT {
            self.star_list[i].as_mut().unwrap().set_vertices(&mut buffer_builder);
        }
        profiler_fill.pop();

        profiler_fill.push("update mesh");
        let mesh_data = buffer_builder.build_or_throw();
        self.star_index_count = mesh_data.draw_state().index_count;
        // Note: it would be nice to not have to use a new command buffer.
        seams::write_to_buffer(&self.star_buffer, mesh_data.vertex_buffer());
        profiler_fill.pop();
    }

    pub fn render(&self, render_target: &seams::RenderTarget, pose_stack: &seams::PoseStack, brightness: f32) {
        // uniform values
        let mut view_model_matrix = seams::get_model_view_stack();
        view_model_matrix.push_matrix();
        view_model_matrix.mul(pose_stack.last().pose());

        let matrix = view_model_matrix.copy_matrix();
        let gpu_buffer_slice = seams::get_dynamic_uniforms().write_transform(
            &matrix,
            [brightness, brightness, brightness, brightness],
            seams::Vector3f::zero(),
            seams::Matrix4f::identity(),
        );

        // resources
        let color = render_target.get_color_texture_view();
        let depth = render_target.get_depth_texture_view();

        // render pass
        {
            let mut render_pass = seams::create_render_pass("Nicer Skies Stars", &color, &depth);
            render_pass.set_pipeline(self.star_pipeline);
            seams::bind_default_uniforms(&mut render_pass);
            render_pass.set_uniform("DynamicTransforms", &gpu_buffer_slice);
            render_pass.set_vertex_buffer(0, &self.star_buffer.slice());
            render_pass.set_index_buffer(&self.index_buffer.get_buffer(self.star_index_count), self.index_buffer.type_());
            render_pass.draw_indexed(self.star_index_count, 1, 0, 0, 0);
        }

        view_model_matrix.pop_matrix();
    }
}
