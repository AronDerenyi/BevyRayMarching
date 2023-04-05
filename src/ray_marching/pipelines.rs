use bevy::{
    asset::load_internal_asset,
    core_pipeline::fullscreen_vertex_shader::fullscreen_shader_vertex_state,
    prelude::{
        default, App, Component, FromWorld, HandleUntyped, Mat4, Resource, Shader, Vec3, World,
    },
    reflect::TypeUuid,
    render::{
        render_resource::{ShaderType, *},
        renderer::{RenderDevice, RenderQueue},
    },
};

use super::shaders;

#[derive(Resource, Debug)]
pub struct Pipelines {
    pub filter_bind_layout: BindGroupLayout,
    pub filter_pipeline: CachedRenderPipelineId,
    pub sampler: Sampler,
}

impl FromWorld for Pipelines {
    fn from_world(world: &mut World) -> Self {
        let device = world.resource::<RenderDevice>();

        let filter_bind_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: "Filter bind group layout".into(),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    visibility: ShaderStages::FRAGMENT,
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    visibility: ShaderStages::FRAGMENT,
                    count: None,
                },
            ],
        });

        let sampler = device.create_sampler(&SamplerDescriptor {
            min_filter: FilterMode::Linear,
            mag_filter: FilterMode::Linear,
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            ..Default::default()
        });

        let mut cache = world.resource_mut::<PipelineCache>();
        let filter_pipeline = cache.queue_render_pipeline(RenderPipelineDescriptor {
            label: Some("Filter pipeline".into()),
            layout: vec![filter_bind_layout.clone()],
            push_constant_ranges: vec![],
            vertex: fullscreen_shader_vertex_state(),
            fragment: Some(FragmentState {
                shader: shaders::FILTER_SHADER_HANDLE.typed(),
                shader_defs: default(),
                entry_point: "main".into(),
                targets: vec![Some(ColorTargetState {
                    format: TextureFormat::Rgba8UnormSrgb,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: default(),
            multisample: default(),
            depth_stencil: None,
        });

        Self {
            filter_bind_layout,
            filter_pipeline,
            sampler,
        }
    }
}
