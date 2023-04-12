use std::ops::Deref;

use bevy::{
    ecs::query::QueryItem,
    math::Vec3A,
    prelude::{
        warn, Commands, Component, FromWorld, GlobalTransform, IntoSystemConfig, Mat4, Plugin,
        Query, Res, ResMut, Resource, Vec3,
    },
    render::{
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        render_resource::*,
        renderer::{RenderDevice, RenderQueue},
        RenderApp, RenderSet,
    },
};

pub struct ShapePlugin;

impl Plugin for ShapePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_plugin(ExtractComponentPlugin::<ExtractedShape>::default());
        app.sub_app_mut(RenderApp)
            .init_resource::<ShapesUniformBuffer>()
            .init_resource::<ShapesBindGroupLayout>()
            .add_system(prepare_shapes.in_set(RenderSet::Prepare))
            .add_system(queue_shapes_bind_group.in_set(RenderSet::Queue));
    }
}

#[derive(Component, Clone, Default)]
pub struct Shape {
    pub shape_type: ShapeType,
    pub negative: bool,
}

#[derive(Clone)]
pub enum ShapeType {
    Plane,
    Sphere { radius: f32 },
    Cube { size: Vec3 },
    Union,
    Intersection,
}

impl Default for ShapeType {
    fn default() -> Self {
        ShapeType::Union
    }
}

pub const MAX_PLANES: usize = 4;
pub const MAX_SPHERES: usize = 16;
pub const MAX_CUBES: usize = 16;

#[derive(Component, Clone)]
struct ExtractedShape {
    shape_type: ShapeType,
    negative: bool,
    transform: GlobalTransform,
}

impl ExtractComponent for ExtractedShape {
    type Query = (&'static Shape, &'static GlobalTransform);
    type Filter = ();
    type Out = Self;

    fn extract_component((shape, transform): QueryItem<'_, Self::Query>) -> Option<Self> {
        Some(Self {
            shape_type: shape.shape_type.clone(),
            negative: shape.negative,
            transform: transform.clone(),
        })
    }
}

#[derive(ShaderType, Clone, Default)]
struct ShapesUniform {
    plane_count: u32,
    planes: [Transform; MAX_PLANES],
    sphere_count: u32,
    spheres: [Transform; MAX_SPHERES],
    cube_count: u32,
    cubes: [Transform; MAX_CUBES],
}

#[derive(ShaderType, Clone, Default)]
struct Transform {
    pub inv_transform: Mat4,
    pub min_scale: f32,
}

#[derive(Resource, Default)]
struct ShapesUniformBuffer(UniformBuffer<ShapesUniform>);

fn prepare_shapes(
    shapes: Query<&ExtractedShape>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    mut uniform_buffer: ResMut<ShapesUniformBuffer>,
) {
    let mut plane_index = 0;
    let mut sphere_index = 0;
    let mut cube_index = 0;
    let mut uniform = ShapesUniform::default();

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
        match shape.shape_type {
            ShapeType::Plane => {
                if plane_index < MAX_PLANES {
                    uniform.planes[plane_index] = transform;
                    plane_index += 1;
                } else {
                    warn!("Too many planes are in the scene");
                }
            }
            ShapeType::Sphere { radius: _ } => {
                if sphere_index < MAX_SPHERES {
                    uniform.spheres[sphere_index] = transform;
                    sphere_index += 1;
                } else {
                    warn!("Too many spheres are in the scene");
                }
            }
            ShapeType::Cube { size: _ } => {
                if cube_index < MAX_CUBES {
                    uniform.cubes[cube_index] = transform;
                    cube_index += 1;
                } else {
                    warn!("Too many cubes are in the scene");
                }
            }
            _ => {
                warn!("Unsupported shape type");
            }
        }
    }

    uniform.plane_count = plane_index as u32;
    uniform.sphere_count = sphere_index as u32;
    uniform.cube_count = cube_index as u32;

    uniform_buffer.0.set(uniform);
    uniform_buffer.0.write_buffer(&*device, &*queue);
}

fn point_plane_distance(point: Vec3A, plane1: Vec3A, plane2: Vec3A) -> f32 {
    (point - plane1 * point.dot(plane1) - plane2 * point.dot(plane2)).length()
}

#[derive(Resource)]
pub struct ShapesBindGroupLayout(BindGroupLayout);

impl Deref for ShapesBindGroupLayout {
    type Target = BindGroupLayout;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromWorld for ShapesBindGroupLayout {
    fn from_world(world: &mut bevy::prelude::World) -> Self {
        let device = world.resource::<RenderDevice>();
        Self(device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: "shapes_bind_group_layout".into(),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: Some(ShapesUniform::min_size()),
                },
                count: None,
            }],
        }))
    }
}

#[derive(Resource)]
pub struct ShapesBindGroup(BindGroup);

impl Deref for ShapesBindGroup {
    type Target = BindGroup;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

fn queue_shapes_bind_group(
    mut commands: Commands,
    bind_group_layout: Res<ShapesBindGroupLayout>,
    uniform_buffer: Res<ShapesUniformBuffer>,
    device: Res<RenderDevice>,
) {
    commands.insert_resource(ShapesBindGroup(device.create_bind_group(
        &BindGroupDescriptor {
            label: "shapes_bind_group".into(),
            layout: &bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.0.binding().unwrap(),
            }],
        },
    )));
}
