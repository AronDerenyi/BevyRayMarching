use std::ops::Range;

use crate::ray_marching::shape::ShapeGroupOperation;

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
        let sdf = generate_sdf(&shape_group);
        println!("---------- SDF SHADER CODE ----------");
        println!("{}", sdf);
        println!("-------------------------------------");

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
    let spheres = generate_shapes_sdf(dist.as_str(), operation, "sphere", &group.sphere_index_range);
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

/*
var dist_0 = 0.0;
var dist_1 = 0.0;
dist_1 = max(dist_1, sdf_cube(0, pnt));

var dist_2 = 0.0;
dist_2 = min(dist_2, sdf_plane(0, pnt));
for (var i = 0u; i < 2u; i = i + 1) {
    dist_2 = min(dist_2, sdf_phere(i, pnt));
}

dist_1 = max(dist_1, dist_2);
dist_0 = min(dist_0, dist_1);
*/
