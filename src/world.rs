use std::cell::RefCell;

use cgmath::{Vector2, Vector3};
use winit::dpi::PhysicalSize;

use crate::{
    camera::Camera,
    mesh::InstanceData,
    screen::{Screen, SCREEN_HEIGHT, SCREEN_WIDTH},
};

pub struct World {
    pub camera: Camera,
    pub screen: Screen,
}

impl World {
    pub fn new(surface_size: PhysicalSize<u32>) -> Self {
        let camera = Camera {
            position: Vector3::new(0.0f32, 0.0f32, -1.0f32),
            size: Vector2::new(surface_size.width as f32, surface_size.height as f32),
        };

        Self {
            camera,
            screen: Screen::new(),
        }
    }

    pub fn get_instances(&self) -> Vec<InstanceData> {
        let mut instances = Vec::with_capacity(SCREEN_WIDTH as usize * SCREEN_HEIGHT as usize);
        for y in 0..SCREEN_HEIGHT {
            for x in 0..SCREEN_WIDTH {
                if self.screen.pixels[y as usize * SCREEN_WIDTH as usize + x as usize] {
                    instances.push(InstanceData::new(Vector2::new(x as f32, y as f32)));
                }
            }
        }
        instances
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        self.camera.size = Vector2::new(new_size.width as f32, new_size.height as f32);
    }
}
