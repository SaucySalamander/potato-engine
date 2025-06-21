use glam::{Mat4, Vec3};

use crate::engine_loop::input::InputState;

#[derive(Debug)]
pub struct FpsCamera {
    pub position: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub speed: f32,
    pub sensitivity: f32,
}

impl FpsCamera {
    pub fn new(position: Vec3) -> Self {
        Self {
            position,
            yaw: 0.0,
            pitch: 0.0,
            speed: 5.0,
            sensitivity: 0.002,
        }
    }

    pub fn update(&mut self, input: &InputState, delta_time: f32) {
        let forward = self.forward();
        let right = forward.cross(Vec3::Y).normalize();
        let up = right.cross(forward).normalize();

        if input.key_w {
            self.position += forward * self.speed * delta_time;
        }
        if input.key_s {
            self.position -= forward * self.speed * delta_time;
        }
        if input.key_a {
            self.position -= right * self.speed * delta_time;
        }
        if input.key_d {
            self.position += right * self.speed * delta_time;
        }
        if input.key_space {
            self.position += up * self.speed * delta_time;
        }
        if input.key_ctrl {
            self.position -= up * self.speed * delta_time;
        }

        self.yaw += input.mouse_delta_x * self.sensitivity;
        self.pitch -= input.mouse_delta_y * self.sensitivity;
        self.pitch = self
            .pitch
            .clamp(-89.9_f32.to_radians(), 89.9_f32.to_radians());
    }

    pub fn forward(&self) -> Vec3 {
        Vec3::new(
            self.yaw.cos() * self.pitch.cos(),
            self.pitch.sin(),
            self.yaw.sin() * self.pitch.cos(),
        )
        .normalize()
    }

    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(self.position, self.position + self.forward(), Vec3::Y)
    }
}
