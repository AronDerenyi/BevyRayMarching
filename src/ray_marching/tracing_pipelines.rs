use super::{
    camera::CameraBindGroupLayout,
    shaders,
    shape::{ShapeGroup, ShapesBindGroupLayout, MAX_CUBES, MAX_PLANES, MAX_SPHERES},
    stages::StageBindGroupLayouts,
};
use bevy::{
    core_pipeline::fullscreen_vertex_shader::fullscreen_shader_vertex_state,
    prelude::{default, Local, Res, ResMut, Resource},
    render::render_resource::*,
};

#[derive(Resource)]
pub struct TracingPipelines {
    pub first_id: CachedRenderPipelineId,
    pub mid_id: CachedRenderPipelineId,
    pub last_id: CachedRenderPipelineId,
}

impl Default for TracingPipelines {
    fn default() -> Self {
        Self {
            first_id: CachedRenderPipelineId::INVALID,
            mid_id: CachedRenderPipelineId::INVALID,
            last_id: CachedRenderPipelineId::INVALID,
        }
    }
}

pub fn queue_tracing_pipeline(
    mut pipeline: ResMut<TracingPipelines>,
    pipeline_cache: Res<PipelineCache>,
    camera_bind_group_layout: Res<CameraBindGroupLayout>,
    shapes_bind_group_layout: Res<ShapesBindGroupLayout>,
    stage_bind_group_layouts: Res<StageBindGroupLayouts>,
    shape_group: Res<ShapeGroup>,
    mut local_shape_group: Local<Option<ShapeGroup>>,
) {
    let changed = match &*local_shape_group {
        Some(local_shape_group) => *shape_group != *local_shape_group,
        None => true,
    };

    if changed {
        *local_shape_group = Some(shape_group.clone());
        pipeline.first_id = pipeline_cache.queue_render_pipeline(specialized_descriptor(
            "first_tracing_pipeline",
            vec![
                camera_bind_group_layout.clone(),
                shapes_bind_group_layout.clone(),
                stage_bind_group_layouts.first.clone(),
            ],
            vec![
                "FIRST_STAGE".into(),
                ShaderDefVal::Int("MAX_PLANES".into(), MAX_PLANES as i32),
                ShaderDefVal::Int("MAX_SPHERES".into(), MAX_SPHERES as i32),
                ShaderDefVal::Int("MAX_CUBES".into(), MAX_CUBES as i32),
            ],
            TextureFormat::R32Float,
        ));
        pipeline.mid_id = pipeline_cache.queue_render_pipeline(specialized_descriptor(
            "mid_tracing_pipeline",
            vec![
                camera_bind_group_layout.clone(),
                shapes_bind_group_layout.clone(),
                stage_bind_group_layouts.mid.clone(),
            ],
            vec![
                ShaderDefVal::Int("MAX_PLANES".into(), MAX_PLANES as i32),
                ShaderDefVal::Int("MAX_SPHERES".into(), MAX_SPHERES as i32),
                ShaderDefVal::Int("MAX_CUBES".into(), MAX_CUBES as i32),
            ],
            TextureFormat::R32Float,
        ));
        pipeline.last_id = pipeline_cache.queue_render_pipeline(specialized_descriptor(
            "last_tracing_pipeline",
            vec![
                camera_bind_group_layout.clone(),
                shapes_bind_group_layout.clone(),
                stage_bind_group_layouts.last.clone(),
            ],
            vec![
                "LAST_STAGE".into(),
                ShaderDefVal::Int("MAX_PLANES".into(), MAX_PLANES as i32),
                ShaderDefVal::Int("MAX_SPHERES".into(), MAX_SPHERES as i32),
                ShaderDefVal::Int("MAX_CUBES".into(), MAX_CUBES as i32),
            ],
            TextureFormat::Rgba8Unorm,
        ));
    }
}

fn specialized_descriptor(
    label: &'static str,
    layout: Vec<BindGroupLayout>,
    defs: Vec<ShaderDefVal>,
    target_format: TextureFormat,
) -> RenderPipelineDescriptor {
    RenderPipelineDescriptor {
        label: Some(label.into()),
        layout,
        push_constant_ranges: vec![],
        vertex: fullscreen_shader_vertex_state(),
        fragment: Some(FragmentState {
            shader: shaders::TRACING_SHADER_HANDLE.typed(),
            shader_defs: defs,
            entry_point: "main".into(),
            targets: vec![Some(ColorTargetState {
                format: target_format,
                blend: None,
                write_mask: ColorWrites::ALL,
            })],
        }),
        primitive: default(),
        multisample: default(),
        depth_stencil: None,
    }
}
