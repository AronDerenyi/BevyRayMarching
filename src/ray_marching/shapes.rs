use bevy::{
    ecs::query::QueryItem,
    math::Vec3A,
    prelude::{Component, GlobalTransform, Query, Res, ResMut, Vec3, warn},
    render::{
        extract_component::ExtractComponent,
        renderer::{RenderDevice, RenderQueue},
    },
};

use super::pipelines::{Shapes, ShapesMeta, Transform};

#[derive(Component, Clone)]
pub enum Shape {
    Plane,
    Sphere { radius: f32 },
    Cube { size: Vec3 },
}

#[derive(Component, Clone)]
pub(super) struct ExtractedShape {
    shape: Shape,
    transform: GlobalTransform,
}

impl ExtractComponent for ExtractedShape {
    type Query = (&'static Shape, &'static GlobalTransform);
    type Filter = ();
    type Out = Self;

    fn extract_component((shape, transform): QueryItem<'_, Self::Query>) -> Option<Self> {
        Some(Self {
            shape: shape.clone(),
            transform: transform.clone(),
        })
    }
}

fn point_plane_distance(point: Vec3A, plane1: Vec3A, plane2: Vec3A) -> f32 {
    (point - plane1 * point.dot(plane1) - plane2 * point.dot(plane2)).length()
}

pub(super) fn prepare_shapes(
    shapes: Query<&ExtractedShape>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    mut shapes_meta: ResMut<ShapesMeta>,
) {
    let mut plane_set = false;
    let mut sphere_index = 0;
    let mut cube_index = 0;
    let mut gpu_shapes = Shapes::default();

    for shape in shapes.iter() {
        let matrix = shape.transform.affine().matrix3;
        let min_x_scale = point_plane_distance(matrix.x_axis, matrix.y_axis, matrix.z_axis);
        let min_y_scale = point_plane_distance(matrix.y_axis, matrix.x_axis, matrix.z_axis);
        let min_z_scale = point_plane_distance(matrix.z_axis, matrix.x_axis, matrix.y_axis);
        let min_scale = min_x_scale.min(min_y_scale).min(min_z_scale);
        let transform = Transform {
            inv_transform: shape.transform.compute_matrix().inverse(),
            min_scale,
        };
        match shape.shape {
            Shape::Plane => {
                if !plane_set {
                    gpu_shapes.plane = transform;
                    plane_set = true;
                } else {
                    warn!("A plane has already been set");
                }
            }
            Shape::Sphere { radius: _ } => {
                if sphere_index < Shapes::SPHERES {
                    gpu_shapes.spheres[sphere_index] = transform;
                    sphere_index += 1;
                } else {
                    warn!("Too many spheres are in the scene");
                }
            }
            Shape::Cube { size: _ } => {
                if cube_index < Shapes::CUBES {
                    gpu_shapes.cubes[cube_index] = transform;
                    cube_index += 1;
                } else {
                    warn!("Too many cubes are in the scene");
                }
            }
        }
    }

    shapes_meta.set(gpu_shapes);
    shapes_meta.write(&*device, &*queue);
}
