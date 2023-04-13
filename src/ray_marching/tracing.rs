use super::{
    camera::CameraBindGroupLayout,
    shape::{ShapeGroup, ShapesBindGroupLayout, MAX_CUBES, MAX_PLANES, MAX_SPHERES},
    stages::StageBindGroupLayouts,
};
use crate::ray_marching::shape::ShapeGroupOperation;
use bevy::{
    core_pipeline::fullscreen_vertex_shader::fullscreen_shader_vertex_state,
    prelude::{
        default, Assets, EventReader, EventWriter, Handle, IntoSystemAppConfig, IntoSystemConfig,
        Local, Plugin, Res, ResMut, Resource,
    },
    render::{render_resource::*, ExtractSchedule, MainWorld, RenderApp, RenderSet},
};
use std::ops::Range;

const SHADER: &str = include_str!("shaders/tracing.wgsl");

pub struct TracingPlugin;

impl Plugin for TracingPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.sub_app_mut(RenderApp)
            .init_resource::<TracingPipelines>()
            .add_event::<ShaderEvent>()
            .add_system(
                extract_shader
                    .in_schedule(ExtractSchedule)
                    .run_if(invalid_pipelines),
            )
            .add_system(queue_pipelines.in_set(RenderSet::Queue));
    }
}

struct ShaderEvent(Handle<Shader>);

fn invalid_pipelines(pipelines: Res<TracingPipelines>) -> bool {
    pipelines.invalid
}

fn extract_shader(
    mut main_world: ResMut<MainWorld>,
    mut shader_event: EventWriter<ShaderEvent>,
    shape_group: Option<Res<ShapeGroup>>,
) {
    if let Some(shape_group) = shape_group {
        let shader_source = format!("{}\n{}", SHADER, generate_sdf(&shape_group));
        let mut shaders = main_world.resource_mut::<Assets<Shader>>();
        shader_event.send(ShaderEvent(shaders.add(Shader::from_wgsl(shader_source))));
    }
}

#[derive(Resource)]
pub struct TracingPipelines {
    pub invalid: bool,
    pub first_id: CachedRenderPipelineId,
    pub mid_id: CachedRenderPipelineId,
    pub last_id: CachedRenderPipelineId,
}

impl Default for TracingPipelines {
    fn default() -> Self {
        Self {
            invalid: true,
            first_id: CachedRenderPipelineId::INVALID,
            mid_id: CachedRenderPipelineId::INVALID,
            last_id: CachedRenderPipelineId::INVALID,
        }
    }
}

fn queue_pipelines(
    mut pipelines: ResMut<TracingPipelines>,
    pipeline_cache: Res<PipelineCache>,
    camera_bind_group_layout: Res<CameraBindGroupLayout>,
    shapes_bind_group_layout: Res<ShapesBindGroupLayout>,
    stage_bind_group_layouts: Res<StageBindGroupLayouts>,
    shape_group: Res<ShapeGroup>,
    mut local_shape_group: Local<Option<ShapeGroup>>,
    mut shader_event: EventReader<ShaderEvent>,
) {
    let changed = match &*local_shape_group {
        Some(local_shape_group) => *shape_group != *local_shape_group,
        None => true,
    };

    if changed {
        *local_shape_group = Some(shape_group.clone());
        pipelines.invalid = true;
        pipelines.first_id = CachedRenderPipelineId::INVALID;
        pipelines.mid_id = CachedRenderPipelineId::INVALID;
        pipelines.last_id = CachedRenderPipelineId::INVALID;
    } else if let Some(ShaderEvent(handle)) = shader_event.iter().last() {
        pipelines.invalid = false;
        pipelines.first_id = pipeline_cache.queue_render_pipeline(specialized_descriptor(
            "first_tracing_pipeline",
            vec![
                camera_bind_group_layout.clone(),
                shapes_bind_group_layout.clone(),
                stage_bind_group_layouts.first.clone(),
            ],
            handle.clone(),
            vec![
                "FIRST_STAGE".into(),
                ShaderDefVal::Int("MAX_PLANES".into(), MAX_PLANES as i32),
                ShaderDefVal::Int("MAX_SPHERES".into(), MAX_SPHERES as i32),
                ShaderDefVal::Int("MAX_CUBES".into(), MAX_CUBES as i32),
            ],
            TextureFormat::R32Float,
        ));
        pipelines.mid_id = pipeline_cache.queue_render_pipeline(specialized_descriptor(
            "mid_tracing_pipeline",
            vec![
                camera_bind_group_layout.clone(),
                shapes_bind_group_layout.clone(),
                stage_bind_group_layouts.mid.clone(),
            ],
            handle.clone(),
            vec![
                ShaderDefVal::Int("MAX_PLANES".into(), MAX_PLANES as i32),
                ShaderDefVal::Int("MAX_SPHERES".into(), MAX_SPHERES as i32),
                ShaderDefVal::Int("MAX_CUBES".into(), MAX_CUBES as i32),
            ],
            TextureFormat::R32Float,
        ));
        pipelines.last_id = pipeline_cache.queue_render_pipeline(specialized_descriptor(
            "last_tracing_pipeline",
            vec![
                camera_bind_group_layout.clone(),
                shapes_bind_group_layout.clone(),
                stage_bind_group_layouts.last.clone(),
            ],
            handle.clone(),
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
    shader: Handle<Shader>,
    defs: Vec<ShaderDefVal>,
    target_format: TextureFormat,
) -> RenderPipelineDescriptor {
    RenderPipelineDescriptor {
        label: Some(label.into()),
        layout,
        push_constant_ranges: vec![],
        vertex: fullscreen_shader_vertex_state(),
        fragment: Some(FragmentState {
            shader,
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

fn generate_sdf(group: &ShapeGroup) -> String {
    let mut group_index = 0u8;
    let (source, dist) = generate_group_sdf(group, &mut group_index);

    format!(
        "fn sdf(pnt: vec3<f32>) -> f32 {{
{source}return {dist};
}}"
    )
}

fn generate_group_sdf(group: &ShapeGroup, index: &mut u8) -> (String, String) {
    let dist = format!("dist_{}", index);
    let operation = match group.operation {
        ShapeGroupOperation::Min => "min",
        ShapeGroupOperation::Max => "max",
    };

    let planes = generate_shapes_sdf(dist.as_str(), operation, "plane", &group.plane_index_range);
    let spheres = generate_shapes_sdf(
        dist.as_str(),
        operation,
        "sphere",
        &group.sphere_index_range,
    );
    let cubes = generate_shapes_sdf(dist.as_str(), operation, "cube", &group.cube_index_range);
    let mut source = match group.operation {
        ShapeGroupOperation::Min => format!("var {dist} = 1024.0;\n{planes}{spheres}{cubes}"),
        ShapeGroupOperation::Max => format!("var {dist} = -1024.0;\n{planes}{spheres}{cubes}"),
    };

    *index += 1;
    for group in group.children.iter() {
        let (group_source, group_dist) = generate_group_sdf(group, index);
        source += &group_source;

        if group.negative {
            source += &format!("{dist} = {operation}({dist}, -{group_dist});\n");
        } else {
            source += &format!("{dist} = {operation}({dist}, {group_dist});\n");
        }
    }

    (source, dist)
}

fn generate_shapes_sdf(
    dist: &str,
    operation: &str,
    shape: &str,
    index_range: &Range<u8>,
) -> String {
    let start_index = index_range.start;
    let end_index = index_range.end;
    match end_index - start_index {
        0 => String::new(),
        1 => format!("{dist} = {operation}({dist}, sdf_{shape}({start_index}u, pnt));\n"),
        _ => format!(
            "for (var i = {start_index}u; i < {end_index}u; i = i + 1u) {{
{dist} = {operation}({dist}, sdf_{shape}(i, pnt));
}}\n"
        ),
    }
}
