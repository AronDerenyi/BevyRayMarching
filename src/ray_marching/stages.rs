use super::RayMarching;
use bevy::{
    prelude::{
        Commands, Component, Entity, FromWorld, IntoSystemConfig, Plugin, Query, Res, ResMut,
        Resource, UVec2, Vec2,
    },
    render::{
        camera::ExtractedCamera,
        render_resource::*,
        renderer::{RenderDevice, RenderQueue},
        texture::{CachedTexture, TextureCache},
        RenderApp, RenderSet,
    },
};

pub struct StagesPlugin;

impl Plugin for StagesPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.sub_app_mut(RenderApp)
            .init_resource::<StageUniformBuffer>()
            .init_resource::<StageBindGroupLayouts>()
            .init_resource::<StageSamplers>()
            .add_system(prepare_stages.in_set(RenderSet::Prepare))
            .add_system(queue_stage_bind_groups.in_set(RenderSet::Queue));
    }
}

#[derive(Component)]
pub struct StageTextures {
    pub first: CachedTexture,
    pub mid: Vec<CachedTexture>,
    pub last: CachedTexture,
}

#[derive(Component)]
pub struct StageIndices {
    pub first: u32,
    pub mid: Vec<u32>,
    pub last: u32,
}

#[derive(ShaderType, Clone, Default)]
struct StageUniform {
    texel_size: Vec2,
}

#[derive(Resource, Default)]
struct StageUniformBuffer(DynamicUniformBuffer<StageUniform>);

fn prepare_stages(
    mut commands: Commands,
    mut texture_cache: ResMut<TextureCache>,
    mut uniform_buffer: ResMut<StageUniformBuffer>,
    cameras: Query<(Entity, &RayMarching, &ExtractedCamera)>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
) {
    uniform_buffer.0.clear();
    let entities = cameras
        .iter()
        .filter_map(|(entity, ray_marching, camera)| {
            if let Some(UVec2 {
                x: width,
                y: height,
            }) = camera.physical_viewport_size
            {
                let width = (width as f32 * ray_marching.resolution_scale) as u32;
                let height = (height as f32 * ray_marching.resolution_scale) as u32;
                let scaling: u32 = ray_marching.resolution_scaling;
                let start = ray_marching
                    .resolution_start
                    .min(width / scaling)
                    .min(height / scaling)
                    .max(1);

                let (start_width, start_height) = if height < width {
                    (
                        ((start * width) as f32 / height as f32).round() as u32,
                        start,
                    )
                } else {
                    (
                        start,
                        ((start * height) as f32 / width as f32).round() as u32,
                    )
                };

                let mid_stages = u32::min(width / start_width, height / start_height).ilog(scaling);

                let descriptor = TextureDescriptor {
                    label: Some("stage_texture"),
                    size: Extent3d {
                        width,
                        height,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: TextureDimension::D2,
                    format: TextureFormat::R32Float,
                    usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                };

                let w = start_width;
                let h = start_height;
                let first_index = uniform_buffer.0.push(StageUniform {
                    texel_size: Vec2::new(1.0 / w as f32, 1.0 / h as f32),
                });
                let first_texture = texture_cache.get(
                    &device,
                    TextureDescriptor {
                        size: Extent3d {
                            width: w,
                            height: h,
                            depth_or_array_layers: 1,
                        },
                        ..descriptor
                    },
                );

                let mut mid_indices = Vec::<u32>::with_capacity(mid_stages as usize);
                let mut mid_textures = Vec::<CachedTexture>::with_capacity(mid_stages as usize);
                for stage in 1..=mid_stages {
                    let scale = scaling.pow(stage);
                    let w = start_width * scale;
                    let h = start_height * scale;
                    mid_indices.push(uniform_buffer.0.push(StageUniform {
                        texel_size: Vec2::new(1.0 / w as f32, 1.0 / h as f32),
                    }));
                    mid_textures.push(texture_cache.get(
                        &device,
                        TextureDescriptor {
                            size: Extent3d {
                                width: w,
                                height: h,
                                depth_or_array_layers: 1,
                            },
                            ..descriptor
                        },
                    ));
                }

                let last_index = uniform_buffer.0.push(StageUniform {
                    texel_size: Vec2::new(1.0 / width as f32, 1.0 / height as f32),
                });
                let last_texture = texture_cache.get(
                    &device,
                    TextureDescriptor {
                        size: Extent3d {
                            width,
                            height,
                            depth_or_array_layers: 1,
                        },
                        format: TextureFormat::Rgba8Unorm,
                        ..descriptor
                    },
                );

                Some((
                    entity,
                    (
                        StageTextures {
                            first: first_texture,
                            mid: mid_textures,
                            last: last_texture,
                        },
                        StageIndices {
                            first: first_index,
                            mid: mid_indices,
                            last: last_index,
                        },
                    ),
                ))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    commands.insert_or_spawn_batch(entities);
    uniform_buffer.0.write_buffer(&*device, &*queue);
}

#[derive(Resource)]
pub struct StageBindGroupLayouts {
    pub first: BindGroupLayout,
    pub last: BindGroupLayout,
    pub mid: BindGroupLayout,
    pub upsampling: BindGroupLayout,
}

impl FromWorld for StageBindGroupLayouts {
    fn from_world(world: &mut bevy::prelude::World) -> Self {
        let device = world.resource::<RenderDevice>();
        Self {
            first: device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: "first_stage_bind_group_layout".into(),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        min_binding_size: Some(StageUniform::min_size()),
                    },
                    count: None,
                }],
            }),
            mid: device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: "mid_stage_bind_group_layout".into(),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: true,
                            min_binding_size: Some(StageUniform::min_size()),
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: true },
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Sampler(SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            }),
            last: device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: "last_stage_bind_group_layout".into(),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: true,
                            min_binding_size: Some(StageUniform::min_size()),
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: true },
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Sampler(SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            }),
            upsampling: device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: "upsampling_stage_bind_group_layout".into(),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: true },
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Sampler(SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            }),
        }
    }
}

#[derive(Resource)]
struct StageSamplers {
    stage: Sampler,
    upsampling: Sampler,
}

impl FromWorld for StageSamplers {
    fn from_world(world: &mut bevy::prelude::World) -> Self {
        let device = world.resource::<RenderDevice>();
        Self {
            stage: device.create_sampler(&SamplerDescriptor {
                min_filter: FilterMode::Nearest,
                mag_filter: FilterMode::Nearest,
                address_mode_u: AddressMode::ClampToEdge,
                address_mode_v: AddressMode::ClampToEdge,
                ..Default::default()
            }),
            upsampling: device.create_sampler(&SamplerDescriptor {
                min_filter: FilterMode::Linear,
                mag_filter: FilterMode::Linear,
                address_mode_u: AddressMode::ClampToEdge,
                address_mode_v: AddressMode::ClampToEdge,
                ..Default::default()
            }),
        }
    }
}

#[derive(Component)]
pub struct StageBindGroups {
    pub first: BindGroup,
    pub mid: Vec<BindGroup>,
    pub last: BindGroup,
    pub upsampling: BindGroup,
}

fn queue_stage_bind_groups(
    mut commands: Commands,
    entities: Query<(Entity, &StageTextures)>,
    uniform_buffer: Res<StageUniformBuffer>,
    bind_group_layouts: Res<StageBindGroupLayouts>,
    samplers: Res<StageSamplers>,
    device: Res<RenderDevice>,
) {
    let entities = entities
        .iter()
        .map(|(entity, textures)| {
            let first_bind_group = device.create_bind_group(&BindGroupDescriptor {
                label: "first_stage_bind_group".into(),
                layout: &bind_group_layouts.first,
                entries: &[BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.0.binding().unwrap(),
                }],
            });

            let mut prev_texture = &textures.first;
            let mid_bind_groups = textures
                .mid
                .iter()
                .map(|texture| {
                    let mid_bind_group = device.create_bind_group(&BindGroupDescriptor {
                        label: "mid_stage_bind_group".into(),
                        layout: &bind_group_layouts.mid,
                        entries: &[
                            BindGroupEntry {
                                binding: 0,
                                resource: uniform_buffer.0.binding().unwrap(),
                            },
                            BindGroupEntry {
                                binding: 1,
                                resource: BindingResource::TextureView(&prev_texture.default_view),
                            },
                            BindGroupEntry {
                                binding: 2,
                                resource: BindingResource::Sampler(&samplers.stage),
                            },
                        ],
                    });
                    prev_texture = &texture;
                    mid_bind_group
                })
                .collect::<Vec<_>>();

            let last_bind_group = device.create_bind_group(&BindGroupDescriptor {
                label: "last_stage_bind_group".into(),
                layout: &bind_group_layouts.last,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: uniform_buffer.0.binding().unwrap(),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: BindingResource::TextureView(&prev_texture.default_view),
                    },
                    BindGroupEntry {
                        binding: 2,
                        resource: BindingResource::Sampler(&samplers.stage),
                    },
                ],
            });

            let upsampling_bind_group = device.create_bind_group(&BindGroupDescriptor {
                label: "upsampling_stage_bind_group".into(),
                layout: &bind_group_layouts.upsampling,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::TextureView(&textures.last.default_view),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: BindingResource::Sampler(&samplers.upsampling),
                    },
                ],
            });

            (
                entity,
                StageBindGroups {
                    first: first_bind_group,
                    mid: mid_bind_groups,
                    last: last_bind_group,
                    upsampling: upsampling_bind_group,
                },
            )
        })
        .collect::<Vec<_>>();

    commands.insert_or_spawn_batch(entities);
}
