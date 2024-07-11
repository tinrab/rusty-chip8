use cgmath::{prelude::*, Matrix4, Vector2, Vector3};

use crate::screen::{SCREEN_HEIGHT, SCREEN_WIDTH};

pub struct Camera {
    pub position: Vector3<f32>,
    pub size: Vector2<f32>,
    // pub aspect: f32,
    // pub scale: f32,
}

impl Camera {
    pub fn view_projection_matrix(&self) -> cgmath::Matrix4<f32> {
        let view = Matrix4::from_translation(self.position);
        let proj = cgmath::ortho(
            0.0f32,
            SCREEN_WIDTH as f32,
            SCREEN_HEIGHT as f32,
            0.0f32,
            // 0.0f32,
            // self.scale,
            // 0.0f32,
            // self.scale / self.aspect,

            // -0.5f32 * (self.aspect * self.scale),
            // 0.5f32 * (self.aspect * self.scale),
            // -0.5f32 * (1.0f32 / self.aspect) * self.scale,
            // 0.5f32 * (1.0f32 / self.aspect) * self.scale,
            -1.0f32,
            1.0f32,
        );
        proj * view
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    view_projection: [[f32; 4]; 4],
}

impl CameraUniform {
    pub fn new() -> Self {
        Self {
            view_projection: Matrix4::identity().into(),
        }
    }

    pub fn update(&mut self, camera: &Camera) {
        self.view_projection = camera.view_projection_matrix().into();
    }
}
