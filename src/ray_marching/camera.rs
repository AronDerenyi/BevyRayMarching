use bevy::{
    ecs::query::QueryItem,
    prelude::{
        Commands, Component, Entity, FromWorld, GlobalTransform, Projection,
        Projection::Perspective, Query, Res, ResMut, Resource, Vec3, Plugin, IntoSystemConfig,
    },
    render::{
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        render_resource::*,
        renderer::{RenderDevice, RenderQueue}, RenderApp, RenderSet,
    },
};
use std::ops::Deref;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_plugin(ExtractComponentPlugin::<ExtractedCamera>::default());
        app.sub_app_mut(RenderApp)
            .init_resource::<CameraUniformBuffer>()
            .init_resource::<CameraBindGroupLayout>()
            .add_system(prepare_cameras.in_set(RenderSet::Prepare))
            .add_system(queue_camera_bind_group.in_set(RenderSet::Queue));
    }
}

#[derive(Component, Clone)]
struct ExtractedCamera {
    projection: Projection,
    transform: GlobalTransform,
}

impl ExtractComponent for ExtractedCamera {
    type Query = (&'static Projection, &'static GlobalTransform);
    type Filter = ();
    type Out = Self;

    fn extract_component((projection, transform): QueryItem<'_, Self::Query>) -> Option<Self> {
        Some(Self {
            projection: projection.clone(),
            transform: transform.clone(),
        })
    }
}

#[derive(ShaderType, Clone, Default)]
struct CameraUniform {
    position: Vec3,
    right: Vec3,
    up: Vec3,
    forward: Vec3,
}

#[derive(Resource, Default)]
struct CameraUniformBuffer(DynamicUniformBuffer<CameraUniform>);

#[derive(Component)]
pub struct CameraUniformIndex(u32);

impl CameraUniformIndex {
    #[inline]
    pub fn index(&self) -> u32 {
        self.0
    }
}

fn prepare_cameras(
    mut commands: Commands,
    mut uniform_buffer: ResMut<CameraUniformBuffer>,
    cameras: Query<(Entity, &ExtractedCamera)>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
) {
    uniform_buffer.0.clear();
    let entities = cameras
        .iter()
        .map(|(entity, camera)| match &camera.projection {
            Perspective(projection) => {
                let fov_tan = (projection.fov * 0.5).tan();
                let matrix = camera.transform.affine().matrix3;
                let uniform = CameraUniform {
                    position: camera.transform.translation(),
                    right: matrix * Vec3::X * fov_tan * projection.aspect_ratio,
                    up: matrix * Vec3::Y * fov_tan,
                    forward: matrix * Vec3::NEG_Z,
                };
                (entity, CameraUniformIndex(uniform_buffer.0.push(uniform)))
            }
            _ => panic!("Unsupported projection"),
        })
        .collect::<Vec<_>>();

    commands.insert_or_spawn_batch(entities);
    uniform_buffer.0.write_buffer(&*device, &*queue);
}

#[derive(Resource)]
pub struct CameraBindGroupLayout(BindGroupLayout);

impl Deref for CameraBindGroupLayout {
    type Target = BindGroupLayout;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromWorld for CameraBindGroupLayout {
    fn from_world(world: &mut bevy::prelude::World) -> Self {
        let device = world.resource::<RenderDevice>();
        Self(device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: "camera_bind_group_layout".into(),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: Some(CameraUniform::min_size()),
                },
                count: None,
            }],
        }))
    }
}

#[derive(Resource)]
pub struct CameraBindGroup(BindGroup);

impl Deref for CameraBindGroup {
    type Target = BindGroup;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

fn queue_camera_bind_group(
    mut commands: Commands,
    bind_group_layout: Res<CameraBindGroupLayout>,
    uniform_buffer: Res<CameraUniformBuffer>,
    device: Res<RenderDevice>,
) {
    commands.insert_resource(CameraBindGroup(device.create_bind_group(
        &BindGroupDescriptor {
            label: "camera_bind_group".into(),
            layout: &bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.0.binding().unwrap(),
            }],
        },
    )));
}
