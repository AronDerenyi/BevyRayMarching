use bevy::{prelude::Vec3, render::render_resource::Extent3d};

use crate::ray_marching::ShapeImage;

use super::Model;

pub fn build(model: &Model, resolution: Extent3d, padding: u32) -> ShapeImage {
    let size = model.max() - model.min();
    let padding = Vec3::new(
        size.x * padding as f32 / (resolution.width - 2 * padding - 1) as f32,
        size.y * padding as f32 / (resolution.height - 2 * padding - 1) as f32,
        size.z * padding as f32 / (resolution.depth_or_array_layers - 2 * padding - 1) as f32,
    );

    let size = model.max() - model.min() + padding * 2.0;
    let offset = model.min() - padding;

    let mut data = Vec::with_capacity(
        (resolution.width * resolution.height * resolution.depth_or_array_layers) as usize,
    );

    for z in 0..resolution.width {
        for y in 0..resolution.height {
            for x in 0..resolution.depth_or_array_layers {
                let pnt = Vec3::new(
                    x as f32 / (resolution.width - 1) as f32,
                    y as f32 / (resolution.height - 1) as f32,
                    z as f32 / (resolution.depth_or_array_layers - 1) as f32,
                ) * size
                    + offset;
                data.push(model.distance(pnt));
            }
            println!("============= {y} {z} =============");
        }
    }

    ShapeImage {
        size,
        resolution,
        data,
    }
}
