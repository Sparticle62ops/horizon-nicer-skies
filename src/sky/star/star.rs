//! 1:1 port of Star.

use crate::seams;

const RAD_DIFF: f32 = 0.15;

pub struct Star {
    x_coord: f32,
    y_coord: f32,
    z_coord: f32,

    longitude_sin: f32,
    longitude_cos: f32,

    latitude_sin: f32,
    latitude_cos: f32,

    resize_speed: f32,
    spin_speed: f32,

    radius: f32,
    color: i32,

    current_angle: f32,
    current_radius: f32,
}

impl Star {
    pub fn new(rand_x: f32, rand_y: f32, rand_z: f32, size: f32, color: i32, resize_speed: f32, spin_speed: f32) -> Self {
        let invsqrt_distance = seams::mth_inv_sqrt(rand_x * rand_x + rand_y * rand_y + rand_z * rand_z);
        let x_coord = rand_x * invsqrt_distance * 100.0;
        let y_coord = rand_y * invsqrt_distance * 100.0;
        let z_coord = rand_z * invsqrt_distance * 100.0;

        let polar_angle = (rand_x as f64).atan2(rand_z as f64);
        let longitude_sin = polar_angle.sin() as f32;
        let longitude_cos = polar_angle.cos() as f32;

        let proj = (rand_x as f64).hypot(rand_z as f64).atan2(rand_y as f64);
        let latitude_sin = proj.sin() as f32;
        let latitude_cos = proj.cos() as f32;

        Star {
            color,
            x_coord,
            y_coord,
            z_coord,
            longitude_sin,
            longitude_cos,
            latitude_sin,
            latitude_cos,
            spin_speed,
            resize_speed,
            radius: size,
            current_radius: size,
            // random angle from 0 to 2π
            current_angle: (spin_speed + 0.01) * 628.3,
        }
    }

    pub fn tick(&mut self, ticks: i32) {
        self.current_angle += self.spin_speed;
        self.current_radius = self.radius + (((ticks as f32) * self.resize_speed).sin()) * RAD_DIFF;
    }

    pub fn set_vertices(&mut self, buffer_builder: &mut seams::BufferBuilder) {
        let horizontal_vertex_distance = self.current_radius * self.current_angle.cos();
        let vertical_vertex_distance = self.current_radius * self.current_angle.sin();

        let first_vertex_distance = -horizontal_vertex_distance + vertical_vertex_distance;
        let second_vertex_distance = -horizontal_vertex_distance - vertical_vertex_distance;
        let third_vertex_distance = horizontal_vertex_distance - vertical_vertex_distance;
        let fourth_vertex_distance = horizontal_vertex_distance + vertical_vertex_distance;

        let ae1 = third_vertex_distance * self.latitude_cos;
        let ae2 = fourth_vertex_distance * self.latitude_cos;
        let ae3 = first_vertex_distance * self.latitude_cos;
        let ae4 = second_vertex_distance * self.latitude_cos;

        buffer_builder
            .add_vertex(
                self.x_coord + ae1 * self.longitude_sin - second_vertex_distance * self.longitude_cos,
                self.y_coord + first_vertex_distance * self.latitude_sin,
                self.z_coord + second_vertex_distance * self.longitude_sin + ae1 * self.longitude_cos,
            )
            .set_color(self.color)
            .add_vertex(
                self.x_coord + ae2 * self.longitude_sin - third_vertex_distance * self.longitude_cos,
                self.y_coord + second_vertex_distance * self.latitude_sin,
                self.z_coord + third_vertex_distance * self.longitude_sin + ae2 * self.longitude_cos,
            )
            .set_color(self.color)
            .add_vertex(
                self.x_coord + ae3 * self.longitude_sin - fourth_vertex_distance * self.longitude_cos,
                self.y_coord + third_vertex_distance * self.latitude_sin,
                self.z_coord + fourth_vertex_distance * self.longitude_sin + ae3 * self.longitude_cos,
            )
            .set_color(self.color)
            .add_vertex(
                self.x_coord + ae4 * self.longitude_sin - first_vertex_distance * self.longitude_cos,
                self.y_coord + fourth_vertex_distance * self.latitude_sin,
                self.z_coord + first_vertex_distance * self.longitude_sin + ae4 * self.longitude_cos,
            )
            .set_color(self.color);
    }
}
