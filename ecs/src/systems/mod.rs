use glam::Vec3;

use crate::{components::{Camera, FpsCamera, Position}, input::InputState, World};

pub fn update_fps_camera_system(world: &mut World, input: &InputState, delta_time: f32) {
        for (camera, pos, _) in world.query::<(&mut FpsCamera, &mut Position, &Camera)>() {
        let forward = Vec3::new(
            camera.yaw.cos() * camera.pitch.cos(),
            camera.pitch.sin(),
            camera.yaw.sin() * camera.pitch.cos(),
        )
        .normalize();
        let right = forward.cross(Vec3::Y).normalize();
        let up = right.cross(forward).normalize();

        // Movement
        let mut velocity = Vec3::ZERO;
        if input.key_w {
            velocity += forward;
        }
        if input.key_s {
            velocity -= forward;
        }
        if input.key_d {
            velocity += right;
        }
        if input.key_a {
            velocity -= right;
        }
        if input.key_space {
            velocity += up;
        }
        if input.key_ctrl {
            velocity -= up;
        }

        if velocity.length_squared() > 0.0 {
            *pos = Position(pos.0 + velocity.normalize() * camera.speed * delta_time);
        }

        camera.yaw += input.mouse_delta_x * camera.sensitivity;
        camera.pitch -= input.mouse_delta_y * camera.sensitivity;
        camera.pitch = camera
            .pitch
            .clamp(-89.9_f32.to_radians(), 89.9_f32.to_radians());
    }
}