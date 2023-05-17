use bevy::{
    ecs::{
        query::QueryItem,
        system::{lifetimeless::SRes, SystemParamItem},
    },
    math::Vec3A,
    prelude::{
        default, warn, AddAsset, Children, Commands, Component, Deref, Entity, FromWorld,
        GlobalTransform, Handle, IntoSystemConfig, Mat4, Parent, Plugin, Query, Res, ResMut,
        Resource, Vec3, With, Without,
    },
    reflect::{FromReflect, Reflect, TypeUuid},
    render::{
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        render_asset::{PrepareAssetError, RenderAsset, RenderAssetPlugin, RenderAssets},
        render_resource::*,
        renderer::{RenderDevice, RenderQueue},
        RenderApp, RenderSet,
    },
    utils::HashMap,
};
use std::{
    borrow::Borrow,
    ops::{Deref, Range},
};

pub struct ShapePlugin;

impl Plugin for ShapePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_asset::<ShapeImage>()
            .add_plugin(ExtractComponentPlugin::<ExtractedShape>::default())
            .add_plugin(ExtractComponentPlugin::<RootShape>::default())
            .add_plugin(RenderAssetPlugin::<ShapeImage>::default());
        app.sub_app_mut(RenderApp)
            .init_resource::<ShapesUniformBuffer>()
            .init_resource::<ShapesBindGroupLayout>()
            .init_resource::<ShapeSampler>()
            .init_resource::<ShapeImages>()
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
    Image(Handle<ShapeImage>),
}

#[derive(Reflect, FromReflect, Debug, Clone, TypeUuid)]
#[uuid = "ffded854-09c2-4261-835a-ee6f20a96ad9"]
#[reflect_value]
pub struct ShapeImage {
    pub size: Vec3,
    pub resolution: Extent3d,
    pub data: Vec<f32>,
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

#[derive(Debug, Clone)]
pub struct ShapeTexture {
    size: Vec3,
    resolution: Extent3d,
    texture: Texture,
    texture_view: TextureView,
}

impl RenderAsset for ShapeImage {
    type ExtractedAsset = ShapeImage;
    type PreparedAsset = ShapeTexture;
    type Param = (SRes<RenderDevice>, SRes<RenderQueue>);

    fn extract_asset(&self) -> Self::ExtractedAsset {
        self.clone()
    }

    fn prepare_asset(
        image: Self::ExtractedAsset,
        (device, queue): &mut SystemParamItem<Self::Param>,
    ) -> Result<Self::PreparedAsset, PrepareAssetError<Self::ExtractedAsset>> {
        let texture = device.create_texture_with_data(
            queue,
            &TextureDescriptor {
                label: "shape_texture".into(),
                size: image.resolution.clone(),
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D3,
                format: TextureFormat::R32Float,
                usage: TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
            unsafe {
                std::slice::from_raw_parts(image.data.as_ptr() as *const u8, image.data.len() * 4)
            },
        );

        let texture_view = texture.create_view(&TextureViewDescriptor::default());

        Ok(ShapeTexture {
            size: image.size,
            resolution: image.resolution,
            texture,
            texture_view,
        })
    }
}

pub const MAX_PLANES: u8 = 4;
pub const MAX_SPHERES: u8 = 24;
pub const MAX_CUBES: u8 = 24;
pub const MAX_IMAGES: u8 = 4;

pub const MAX_TEXTURES: u8 = 2;

#[derive(ShaderType, Clone, Default)]
struct ShapesUniform {
    planes: [Plane; MAX_PLANES as usize],
    spheres: [Sphere; MAX_SPHERES as usize],
    cubes: [Cube; MAX_CUBES as usize],
    images: [Image; MAX_IMAGES as usize],
    texture_properties: [TextureProperties; MAX_TEXTURES as usize],
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
    pub bounds: Vec3,
    pub inv_transform: Mat4,
    pub scale: f32,
    pub material: Material,
}

#[derive(ShaderType, Clone, Default)]
struct Image {
    inv_transform: Mat4,
    scale: f32,
    material: Material,
}

#[derive(ShaderType, Clone, Default)]
struct TextureProperties {
    bounds: Vec3,
    texture_bounds: Vec3,
}

#[derive(Resource, Default)]
struct ShapesUniformBuffer(UniformBuffer<ShapesUniform>);

#[derive(Resource, PartialEq, Eq, Clone, Hash)]
pub struct ShapeGroup {
    pub plane_index_range: Range<u8>,
    pub sphere_index_range: Range<u8>,
    pub cube_index_range: Range<u8>,
    pub image_index_range: Range<u8>,
    pub children: Vec<Self>,
    pub operation: Operation,
    pub negative: bool,
}

#[derive(Default)]
struct ShapeIndices {
    plane: u8,
    sphere: u8,
    cube: u8,
    image: u8,
}

fn prepare_shapes(
    mut commands: Commands,
    roots: Query<Entity, With<RootShape>>,
    shapes: Query<&ExtractedShape>,
    images: Res<RenderAssets<ShapeImage>>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    mut uniform_buffer: ResMut<ShapesUniformBuffer>,
    mut shape_images: ResMut<ShapeImages>,
) {
    shape_images.handles.clear();
    shape_images.indices.clear();

    let mut uniform = ShapesUniform::default();
    let mut indices = ShapeIndices::default();

    let root_group = create_group(
        &shapes,
        &ExtractedShape::default(),
        roots.iter(),
        &mut uniform,
        &mut shape_images,
        &mut indices,
    );
    commands.insert_resource(root_group);

    for (index, handle) in shape_images.handles.iter().enumerate() {
        let texture = match images.get(handle) {
            Some(texture) => texture,
            None => &shape_images.default_texture,
        };

        uniform.texture_properties[index] = TextureProperties {
            bounds: texture.size / 2.0,
            texture_bounds: texture.size
                / 2.0
                / Vec3::new(
                    1.0 - 1.0 / texture.resolution.width as f32,
                    1.0 - 1.0 / texture.resolution.height as f32,
                    1.0 - 1.0 / texture.resolution.depth_or_array_layers as f32,
                ),
        }
    }

    uniform_buffer.0.set(uniform);
    uniform_buffer.0.write_buffer(&*device, &*queue);
}

fn create_group<T>(
    shapes: &Query<&ExtractedShape>,
    shape: &ExtractedShape,
    children: T,
    uniform: &mut ShapesUniform,
    images: &mut ShapeImages,
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
    let mut image_index_range = indices.image..indices.image;

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
                    uniform, images, indices, transform, primitive, material, false,
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
                        uniform, images, indices, transform, primitive, material, *negative,
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
    image_index_range.end = indices.image;

    // Converting the shapes with children into groups
    let children = groups
        .iter()
        .map(|(shape, children)| create_group(shapes, shape, *children, uniform, images, indices))
        .collect::<Vec<_>>();

    ShapeGroup {
        plane_index_range,
        sphere_index_range,
        cube_index_range,
        image_index_range,
        children,
        operation,
        negative,
    }
}

fn add_primitive(
    uniform: &mut ShapesUniform,
    images: &mut ShapeImages,
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
                    bounds: *size / 2.0,
                    inv_transform,
                    scale,
                    material: material.clone(),
                };
                indices.cube += 1;
            }
        }
        Primitive::Image(image) => {
            if indices.image == MAX_IMAGES {
                warn!("Too many images are in the scene");
            } else {
                let (inv_transform, scale) = get_inverse_transform(transform, negative);
                uniform.images[indices.image as usize] = Image {
                    inv_transform,
                    scale,
                    material: material.clone(),
                };
                images.add_image(indices.image, image);
                indices.image += 1;
            }
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
        let mut entries = vec![
            BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: Some(ShapesUniform::min_size()),
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 1,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Sampler(SamplerBindingType::Filtering),
                count: None,
            },
        ];

        for index in 0..MAX_TEXTURES {
            entries.push(BindGroupLayoutEntry {
                binding: 2 + index as u32,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Texture {
                    sample_type: TextureSampleType::Float { filterable: true },
                    view_dimension: TextureViewDimension::D3,
                    multisampled: false,
                },
                count: None,
            });
        }

        Self(device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: "shapes_bind_group_layout".into(),
            entries: &entries,
        }))
    }
}

#[derive(Resource)]
struct ShapeSampler(Sampler);

impl FromWorld for ShapeSampler {
    fn from_world(world: &mut bevy::prelude::World) -> Self {
        let device = world.resource::<RenderDevice>();
        Self(device.create_sampler(&SamplerDescriptor {
            min_filter: FilterMode::Linear,
            mag_filter: FilterMode::Linear,
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            ..Default::default()
        }))
    }
}

#[derive(Resource)]
pub struct ShapeImages {
    default_texture: ShapeTexture,
    handles: Vec<Handle<ShapeImage>>,
    indices: HashMap<u8, u8>,
}

impl FromWorld for ShapeImages {
    fn from_world(world: &mut bevy::prelude::World) -> Self {
        let device = world.resource::<RenderDevice>();
        let queue = world.resource::<RenderQueue>();

        let resolution = Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture_with_data(
            queue,
            &TextureDescriptor {
                label: "default_shape_texture".into(),
                size: resolution,
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D3,
                format: TextureFormat::R8Unorm,
                usage: TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
            &[0u8],
        );

        let texture_view = texture.create_view(&TextureViewDescriptor::default());

        Self {
            default_texture: ShapeTexture {
                size: Vec3::ZERO,
                resolution,
                texture,
                texture_view,
            },
            handles: default(),
            indices: default(),
        }
    }
}

impl ShapeImages {
    fn add_image(&mut self, image_index: u8, image_handle: &Handle<ShapeImage>) {
        if let Some(index) = self
            .handles
            .iter()
            .position(|handle| handle == image_handle)
        {
            self.indices.insert(image_index, index as u8);
        } else if self.handles.len() < MAX_TEXTURES as usize {
            self.indices.insert(image_index, self.handles.len() as u8);
            self.handles.push(image_handle.clone());
        } else {
            warn!("Too many different image are in the scene");
        }
    }

    pub fn get_image_index(&self, shape_index: u8) -> u8 {
        self.indices[&shape_index]
    }
}

#[derive(Resource, Deref)]
pub struct ShapesBindGroup(BindGroup);

fn queue_shapes_bind_group(
    mut commands: Commands,
    images: Res<RenderAssets<ShapeImage>>,
    bind_group_layout: Res<ShapesBindGroupLayout>,
    uniform_buffer: Res<ShapesUniformBuffer>,
    sampler: Res<ShapeSampler>,
    shape_images: Res<ShapeImages>,
    device: Res<RenderDevice>,
) {
    let mut entries = vec![
        BindGroupEntry {
            binding: 0,
            resource: uniform_buffer.0.binding().unwrap(),
        },
        BindGroupEntry {
            binding: 1,
            resource: BindingResource::Sampler(&sampler.0),
        },
    ];

    for index in 0..MAX_TEXTURES as usize {
        let texture = if index < shape_images.handles.len() {
            match images.get(&shape_images.handles[index]) {
                Some(texture) => texture,
                None => &shape_images.default_texture,
            }
        } else {
            &shape_images.default_texture
        };

        entries.push(BindGroupEntry {
            binding: 2 + index as u32,
            resource: BindingResource::TextureView(&texture.texture_view),
        });
    }

    commands.insert_resource(ShapesBindGroup(device.create_bind_group(
        &BindGroupDescriptor {
            label: "shapes_bind_group".into(),
            layout: &bind_group_layout,
            entries: &entries,
        },
    )));
}
