use super::{
    camera::CameraBindGroupLayout,
    shaders, shape::{ShapesBindGroupLayout, MAX_CUBES, MAX_SPHERES},
};
use bevy::{
    core_pipeline::fullscreen_vertex_shader::fullscreen_shader_vertex_state,
    prelude::{default, Res, ResMut, Resource},
    render::render_resource::*,
};

#[derive(Resource)]
pub struct RayMarchingPipeline {
    pub pipeline_id: CachedRenderPipelineId,
}

impl Default for RayMarchingPipeline {
    fn default() -> Self {
        Self {
            pipeline_id: CachedRenderPipelineId::INVALID,
        }
    }
}

pub fn queue_ray_marching_pipeline(
    mut pipeline: ResMut<RayMarchingPipeline>,
    pipeline_cache: Res<PipelineCache>,
    camera_bind_group_layout: Res<CameraBindGroupLayout>,
    shapes_bing_group_layout: Res<ShapesBindGroupLayout>,
) {
    if pipeline.pipeline_id == CachedRenderPipelineId::INVALID {
        pipeline.pipeline_id = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
            label: Some("ray_marching_pipeline".into()),
            layout: vec![
                camera_bind_group_layout.clone(),
                shapes_bing_group_layout.clone(),
            ],
            push_constant_ranges: vec![],
            vertex: fullscreen_shader_vertex_state(),
            fragment: Some(FragmentState {
                shader: shaders::RAYMARCHING_SHADER_HANDLE.typed(),
                shader_defs: vec![
                    ShaderDefVal::UInt("SPHERES".into(), MAX_SPHERES as u32),
                    ShaderDefVal::UInt("CUBES".into(), MAX_CUBES as u32),
                ],
                entry_point: "main".into(),
                targets: vec![Some(ColorTargetState {
                    format: TextureFormat::Rgba8Unorm,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: default(),
            multisample: default(),
            depth_stencil: None,
        });
    }
}
