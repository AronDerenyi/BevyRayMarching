use bevy::{
    ecs::query::QueryItem,
    math::Vec3A,
    prelude::{
        warn, Children, Commands, Component, Entity, FromWorld, GlobalTransform, IntoSystemConfig,
        Mat4, Parent, Plugin, Query, Res, ResMut, Resource, Vec3, With, Without, Handle, Image,
    },
    render::{
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        render_resource::*,
        renderer::{RenderDevice, RenderQueue},
        RenderApp, RenderSet,
    }, asset::Asset, reflect::{Reflect, FromReflect, TypeUuid},
};
use std::{
    borrow::Borrow,
    ops::{Deref, Range},
};

pub struct ShapePlugin;

impl Plugin for ShapePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_plugin(ExtractComponentPlugin::<ExtractedShape>::default());
        app.add_plugin(ExtractComponentPlugin::<RootShape>::default());
        app.sub_app_mut(RenderApp)
            .init_resource::<ShapesUniformBuffer>()
            .init_resource::<ShapesBindGroupLayout>()
            .add_system(prepare_shapes.in_set(RenderSet::Prepare))
            .add_system(queue_shapes_bind_group.in_set(RenderSet::Queue));
    }
}

#[derive(Component, Clone, Default, Debug)]
pub struct Shape {
    pub shape_type: ShapeType,
    pub negative: bool,
}

#[derive(Clone, PartialEq, Debug)]
pub enum ShapeType {
    Primitive(Primitive, Material),
    Compound(Operation),
}

#[derive(Clone, PartialEq, Debug)]
pub enum Primitive {
    Plane,
    Sphere { radius: f32 },
    Cube { size: Vec3 },
    Image { size: Vec3, image: Handle<ShapeImage> },
}

#[derive(Reflect, FromReflect, Debug, Clone, TypeUuid)]
#[uuid = "ffded854-09c2-4261-835a-ee6f20a96ad9"]
#[reflect_value]
pub struct ShapeImage {
    pub data: Vec<u8>,
    pub size: Extent3d,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Operation {
    Union,
    Intersection,
}

impl Default for ShapeType {
    fn default() -> Self {
        Self::Compound(Operation::Union)
    }
}

#[derive(ShaderType, PartialEq, Clone, Debug)]
pub struct Material {
    pub color: Vec3,
}

impl Default for Material {
    fn default() -> Self {
        Self { color: Vec3::ONE }
    }
}

#[derive(Component, Default)]
struct ExtractedShape {
    shape_type: ShapeType,
    children: Option<Vec<Entity>>,
    negative: bool,
    transform: GlobalTransform,
}

impl ExtractComponent for ExtractedShape {
    type Query = (
        &'static Shape,
        &'static GlobalTransform,
        Option<&'static Children>,
    );
    type Filter = ();
    type Out = Self;

    fn extract_component((shape, transform, children): QueryItem<'_, Self::Query>) -> Option<Self> {
        Some(Self {
            shape_type: shape.shape_type.clone(),
            children: children.map(|children| children.iter().map(|entity| *entity).collect()),
            negative: shape.negative,
            transform: transform.clone(),
        })
    }
}

#[derive(Component, Clone, Default)]
struct RootShape;

impl ExtractComponent for RootShape {
    type Query = ();
    type Filter = (With<Shape>, With<GlobalTransform>, Without<Parent>);
    type Out = Self;

    fn extract_component(_: QueryItem<'_, Self::Query>) -> Option<Self> {
        Some(Self)
    }
}

pub const MAX_PLANES: u8 = 4;
pub const MAX_SPHERES: u8 = 24;
pub const MAX_CUBES: u8 = 24;

#[derive(ShaderType, Clone, Default)]
struct ShapesUniform {
    planes: [Plane; MAX_PLANES as usize],
    spheres: [Sphere; MAX_SPHERES as usize],
    cubes: [Cube; MAX_CUBES as usize],
}

#[derive(ShaderType, Clone, Default)]
struct Plane {
    pub inv_transform: Mat4,
    pub scale: f32,
    pub material: Material,
}

#[derive(ShaderType, Clone, Default)]
struct Sphere {
    pub radius: f32,
    pub inv_transform: Mat4,
    pub scale: f32,
    pub material: Material,
}

#[derive(ShaderType, Clone, Default)]
struct Cube {
    pub size: Vec3,
    pub inv_transform: Mat4,
    pub scale: f32,
    pub material: Material,
}

#[derive(Resource, Default)]
struct ShapesUniformBuffer(UniformBuffer<ShapesUniform>);

#[derive(Resource, PartialEq, Eq, Clone, Hash)]
pub struct ShapeGroup {
    pub plane_index_range: Range<u8>,
    pub sphere_index_range: Range<u8>,
    pub cube_index_range: Range<u8>,
    pub children: Vec<Self>,
    pub operation: Operation,
    pub negative: bool,
}

#[derive(Default)]
struct ShapeIndices {
    plane: u8,
    sphere: u8,
    cube: u8,
}

fn prepare_shapes(
    mut commands: Commands,
    roots: Query<Entity, With<RootShape>>,
    shapes: Query<&ExtractedShape>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    mut uniform_buffer: ResMut<ShapesUniformBuffer>,
) {
    let mut uniform = ShapesUniform::default();
    let mut indices = ShapeIndices::default();

    let root_group = create_group(
        &shapes,
        &ExtractedShape::default(),
        roots.iter(),
        &mut uniform,
        &mut indices,
    );
    commands.insert_resource(root_group);

    uniform_buffer.0.set(uniform);
    uniform_buffer.0.write_buffer(&*device, &*queue);
}

fn create_group<T>(
    shapes: &Query<&ExtractedShape>,
    shape: &ExtractedShape,
    children: T,
    uniform: &mut ShapesUniform,
    indices: &mut ShapeIndices,
) -> ShapeGroup
where
    T: IntoIterator,
    T::Item: Borrow<Entity>,
{
    // Saving the starting indices
    let mut plane_index_range = indices.plane..indices.plane;
    let mut sphere_index_range = indices.sphere..indices.sphere;
    let mut cube_index_range = indices.cube..indices.cube;

    // Calculating the operation and adding the shape if it has one
    let (operation, negative) = {
        let ExtractedShape {
            shape_type,
            transform,
            negative,
            ..
        } = shape;

        match shape_type {
            ShapeType::Primitive(primitive, material) => {
                add_primitive(
                    uniform,
                    indices,
                    transform,
                    primitive,
                    material,
                    false,
                );
                (Operation::Union, *negative)
            }
            ShapeType::Compound(operation) => (*operation, *negative),
        }
    };

    // Adding the shapes that don't have children and saving the ones that do
    let mut groups = Vec::<(&ExtractedShape, &Vec<Entity>)>::new();
    for shape in shapes.iter_many(children) {
        let ExtractedShape {
            children,
            shape_type,
            transform,
            negative,
        } = shape;
        match children {
            None => {
                if let ShapeType::Primitive(primitive, material) = shape_type {
                    add_primitive(
                        uniform,
                        indices,
                        transform,
                        primitive,
                        material,
                        *negative,
                    )
                }
            }
            Some(children) => {
                groups.push((shape, children));
            }
        }
    }

    // Saving the ending indices
    plane_index_range.end = indices.plane;
    sphere_index_range.end = indices.sphere;
    cube_index_range.end = indices.cube;

    // Converting the shapes with children into groups
    let children = groups
        .iter()
        .map(|(shape, children)| {
            create_group(
                shapes,
                shape,
                *children,
                uniform,
                indices,
            )
        })
        .collect::<Vec<_>>();

    ShapeGroup {
        plane_index_range,
        sphere_index_range,
        cube_index_range,
        children,
        operation,
        negative,
    }
}

fn add_primitive(
    uniform: &mut ShapesUniform,
    indices: &mut ShapeIndices,
    transform: &GlobalTransform,
    primitive: &Primitive,
    material: &Material,
    negative: bool,
) {
    match primitive {
        Primitive::Plane => {
            if indices.plane == MAX_PLANES {
                warn!("Too many planes are in the scene");
            } else {
                let (inv_transform, scale) = get_inverse_transform(transform, negative);
                uniform.planes[indices.plane as usize] = Plane {
                    inv_transform,
                    scale,
                    material: material.clone(),
                };
                indices.plane += 1;
            }
        }
        Primitive::Sphere { radius } => {
            if indices.sphere == MAX_SPHERES {
                warn!("Too many spheres are in the scene");
            } else {
                let (inv_transform, scale) = get_inverse_transform(transform, negative);
                uniform.spheres[indices.sphere as usize] = Sphere {
                    radius: *radius,
                    inv_transform,
                    scale,
                    material: material.clone(),
                };
                indices.sphere += 1;
            }
        }
        Primitive::Cube { size } => {
            if indices.cube == MAX_CUBES {
                warn!("Too many cubes are in the scene");
            } else {
                let (inv_transform, scale) = get_inverse_transform(transform, negative);
                uniform.cubes[indices.cube as usize] = Cube {
                    size: *size,
                    inv_transform,
                    scale,
                    material: material.clone(),
                };
                indices.cube += 1;
            }
        }
        Primitive::Image { size: _, image: _ } => {
            warn!("Too many images are in the scene");
        }
    }
}

fn get_inverse_transform(transform: &GlobalTransform, negative: bool) -> (Mat4, f32) {
    let matrix = transform.affine().matrix3;
    let min_x_scale = point_plane_distance(matrix.x_axis, matrix.y_axis, matrix.z_axis);
    let min_y_scale = point_plane_distance(matrix.y_axis, matrix.x_axis, matrix.z_axis);
    let min_z_scale = point_plane_distance(matrix.z_axis, matrix.x_axis, matrix.y_axis);

    (
        transform.compute_matrix().inverse(),
        min_x_scale.min(min_y_scale).min(min_z_scale) * if negative { -1.0 } else { 1.0 },
    )
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
