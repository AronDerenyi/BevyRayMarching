use super::stages::StageBindGroupLayouts;
use bevy::{
    asset::load_internal_asset,
    core_pipeline::fullscreen_vertex_shader::fullscreen_shader_vertex_state,
    prelude::{default, FromWorld, HandleUntyped, Plugin, Resource, Shader},
    reflect::TypeUuid,
    render::{
        render_resource::{
            CachedRenderPipelineId, ColorTargetState, ColorWrites, FragmentState, PipelineCache,
            RenderPipelineDescriptor, TextureFormat,
        },
        RenderApp,
    },
};

pub const SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 698782022341067256);

pub struct UpsamplingPlugin;

impl Plugin for UpsamplingPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        load_internal_asset!(app, SHADER_HANDLE, "upsampling.wgsl", Shader::from_wgsl);
        app.sub_app_mut(RenderApp)
            .init_resource::<UpsamplingPipeline>();
    }
}

#[derive(Resource)]
pub struct UpsamplingPipeline {
    pub id: CachedRenderPipelineId,
}

impl FromWorld for UpsamplingPipeline {
    fn from_world(world: &mut bevy::prelude::World) -> Self {
        let pipeline_cache = world.resource::<PipelineCache>();
        let stage_bind_group_layouts = world.resource::<StageBindGroupLayouts>();
        Self {
            id: pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
                label: Some("upsampling_pipeline".into()),
                layout: vec![stage_bind_group_layouts.upsampling.clone()],
                push_constant_ranges: vec![],
                vertex: fullscreen_shader_vertex_state(),
                fragment: Some(FragmentState {
                    shader: SHADER_HANDLE.typed(),
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
            }),
        }
    }
}
