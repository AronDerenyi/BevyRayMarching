use super::RayMarching;
use bevy::{
    ecs::query::QueryItem,
    prelude::{
        Camera, Commands, Component, Entity, FromWorld, IntoSystemConfig, Plugin, Projection,
        Query, Res, ResMut, Resource, Vec3, With,
    },
    render::{
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        render_resource::*,
        renderer::{RenderDevice, RenderQueue},
        view::ExtractedView,
        RenderApp, RenderSet,
    },
};
use log::warn;
use std::ops::Deref;

pub struct ViewPlugin;

impl Plugin for ViewPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_plugin(ExtractComponentPlugin::<ExtractedProjection>::default());
        app.sub_app_mut(RenderApp)
            .init_resource::<ViewUniformBuffer>()
            .init_resource::<ViewBindGroupLayout>()
            .add_system(prepare_views.in_set(RenderSet::Prepare))
            .add_system(queue_view_bind_group.in_set(RenderSet::Queue));
    }
}

#[derive(Component, Clone)]
struct ExtractedProjection {
    fov: f32,
    aspect_ratio: f32,
}

impl ExtractComponent for ExtractedProjection {
    type Query = &'static Projection;
    type Filter = (With<Camera>, With<RayMarching>);
    type Out = Self;

    fn extract_component(projection: QueryItem<'_, Self::Query>) -> Option<Self> {
        if let Projection::Perspective(projection) = projection {
            Some(Self {
                fov: projection.fov,
                aspect_ratio: projection.aspect_ratio,
            })
        } else {
            warn!("Unsupported projection: {:?}", projection);
            None
        }
    }
}

#[derive(ShaderType, Clone, Default)]
struct ViewUniform {
    position: Vec3,
    right: Vec3,
    up: Vec3,
    forward: Vec3,
}

#[derive(Resource, Default)]
struct ViewUniformBuffer(DynamicUniformBuffer<ViewUniform>);

#[derive(Component)]
pub struct ViewUniformIndex(u32);

impl ViewUniformIndex {
    #[inline]
    pub fn index(&self) -> u32 {
        self.0
    }
}

fn prepare_views(
    mut commands: Commands,
    mut uniform_buffer: ResMut<ViewUniformBuffer>,
    views: Query<(Entity, &ExtractedView, &ExtractedProjection)>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
) {
    uniform_buffer.0.clear();
    let entities = views
        .iter()
        .map(|(entity, view, projection)| {
            let fov_tan = (projection.fov * 0.5).tan();
            let matrix = view.transform.affine().matrix3;
            let uniform = ViewUniform {
                position: view.transform.translation(),
                right: matrix * Vec3::X * fov_tan * projection.aspect_ratio,
                up: matrix * Vec3::Y * fov_tan,
                forward: matrix * Vec3::NEG_Z,
            };
            (entity, ViewUniformIndex(uniform_buffer.0.push(uniform)))
        })
        .collect::<Vec<_>>();

    commands.insert_or_spawn_batch(entities);
    uniform_buffer.0.write_buffer(&*device, &*queue);
}

#[derive(Resource)]
pub struct ViewBindGroupLayout(BindGroupLayout);

impl Deref for ViewBindGroupLayout {
    type Target = BindGroupLayout;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromWorld for ViewBindGroupLayout {
    fn from_world(world: &mut bevy::prelude::World) -> Self {
        let device = world.resource::<RenderDevice>();
        Self(device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: "view_bind_group_layout".into(),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: Some(ViewUniform::min_size()),
                },
                count: None,
            }],
        }))
    }
}

#[derive(Resource)]
pub struct ViewBindGroup(BindGroup);

impl Deref for ViewBindGroup {
    type Target = BindGroup;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

fn queue_view_bind_group(
    mut commands: Commands,
    bind_group_layout: Res<ViewBindGroupLayout>,
    uniform_buffer: Res<ViewUniformBuffer>,
    device: Res<RenderDevice>,
) {
    commands.insert_resource(ViewBindGroup(device.create_bind_group(
        &BindGroupDescriptor {
            label: "view_bind_group".into(),
            layout: &bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.0.binding().unwrap(),
            }],
        },
    )));
}
