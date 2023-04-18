use std::ops::Deref;

use bevy::{
    ecs::query::QueryItem,
    prelude::{
        Camera, Commands, Component, Entity, FromWorld, Plugin, Query, Res, ResMut, Resource, Vec3,
        With, IntoSystemConfig,
    },
    render::{
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        render_resource::{
            BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
            BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType, BufferBindingType,
            DynamicUniformBuffer, ShaderStages, ShaderType,
        },
        renderer::{RenderDevice, RenderQueue}, RenderApp, RenderSet,
    },
};

use super::RayMarching;

pub struct EnvironmentPlugin;

impl Plugin for EnvironmentPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_plugin(ExtractComponentPlugin::<Environment>::default());
        app.sub_app_mut(RenderApp)
            .init_resource::<EnvironmentUniformBuffer>()
            .init_resource::<EnvironmentBindGroupLayout>()
            .add_system(prepare_environments.in_set(RenderSet::Prepare))
            .add_system(queue_environment_bind_group.in_set(RenderSet::Queue));
    }
}

#[derive(Component, ShaderType, Clone, Debug)]
pub struct Environment {
    pub sky: Vec3,
    pub sun_direction: Vec3,
    pub sun_light: Vec3,
}

impl Default for Environment {
    fn default() -> Self {
        Self {
            sky: Vec3::ONE,
            sun_direction: Vec3::Z,
            sun_light: Vec3::ONE,
        }
    }
}

impl ExtractComponent for Environment {
    type Query = Option<&'static Environment>;
    type Filter = (With<Camera>, With<RayMarching>);
    type Out = Self;

    fn extract_component(environment: QueryItem<'_, Self::Query>) -> Option<Self> {
        Some(environment.cloned().unwrap_or_default())
    }
}

#[derive(Resource, Default)]
struct EnvironmentUniformBuffer(DynamicUniformBuffer<Environment>);

#[derive(Component)]
pub struct EnvironmentUniformIndex(u32);

impl EnvironmentUniformIndex {
    #[inline]
    pub fn index(&self) -> u32 {
        self.0
    }
}

fn prepare_environments(
    mut commands: Commands,
    mut uniform_buffer: ResMut<EnvironmentUniformBuffer>,
    views: Query<(Entity, &Environment)>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
) {
    uniform_buffer.0.clear();
    let entities = views
        .iter()
        .map(|(entity, environment)| {
            (
                entity,
                EnvironmentUniformIndex(uniform_buffer.0.push(environment.clone())),
            )
        })
        .collect::<Vec<_>>();

    commands.insert_or_spawn_batch(entities);
    uniform_buffer.0.write_buffer(&*device, &*queue);
}

#[derive(Resource)]
pub struct EnvironmentBindGroupLayout(BindGroupLayout);

impl Deref for EnvironmentBindGroupLayout {
    type Target = BindGroupLayout;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromWorld for EnvironmentBindGroupLayout {
    fn from_world(world: &mut bevy::prelude::World) -> Self {
        let device = world.resource::<RenderDevice>();
        Self(device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: "environment_bind_group_layout".into(),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: Some(Environment::min_size()),
                },
                count: None,
            }],
        }))
    }
}

#[derive(Resource)]
pub struct EnvironmentBindGroup(BindGroup);

impl Deref for EnvironmentBindGroup {
    type Target = BindGroup;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

fn queue_environment_bind_group(
    mut commands: Commands,
    bind_group_layout: Res<EnvironmentBindGroupLayout>,
    uniform_buffer: Res<EnvironmentUniformBuffer>,
    device: Res<RenderDevice>,
) {
    commands.insert_resource(EnvironmentBindGroup(device.create_bind_group(
        &BindGroupDescriptor {
            label: "environment_bind_group".into(),
            layout: &bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.0.binding().unwrap(),
            }],
        },
    )));
}
