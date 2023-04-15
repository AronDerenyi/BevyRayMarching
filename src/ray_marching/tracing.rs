use super::{
    camera::CameraBindGroupLayout,
    shape::{ShapeGroup, ShapesBindGroupLayout, MAX_CUBES, MAX_PLANES, MAX_SPHERES},
    stages::StageBindGroupLayouts,
};
use crate::ray_marching::shape::ShapeGroupOperation;
use bevy::{
    core_pipeline::fullscreen_vertex_shader::fullscreen_shader_vertex_state,
    prelude::{
        default, Assets, EventReader, EventWriter, FromWorld, Handle, IntoSystemAppConfig,
        IntoSystemConfig, Local, Plugin, Res, ResMut, Resource,
    },
    render::{render_resource::*, ExtractSchedule, MainWorld, RenderApp, RenderSet},
    utils::HashMap,
};
use std::ops::{Deref, DerefMut, Range};

const SHADER_SOURCE: &str = include_str!("tracing.wgsl");

pub struct TracingPlugin;

impl Plugin for TracingPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.sub_app_mut(RenderApp)
            .init_resource::<ShaderCache>()
            .init_resource::<TracingPipeline>()
            .init_resource::<SpecializedRenderPipelines<TracingPipeline>>()
            .init_resource::<TracingPipelines>()
            .add_system(extract_shader.in_schedule(ExtractSchedule))
            .add_system(queue_pipelines.in_set(RenderSet::Queue));
    }
}

#[derive(Resource, Default)]
struct ShaderCache(HashMap<ShapeGroup, Handle<Shader>>);

impl Deref for ShaderCache {
    type Target = HashMap<ShapeGroup, Handle<Shader>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ShaderCache {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

fn extract_shader(
    mut main_world: ResMut<MainWorld>,
    mut shader_cache: ResMut<ShaderCache>,
    shape_group: Option<Res<ShapeGroup>>,
) {
    if let Some(shape_group) = shape_group {
        if !shader_cache.contains_key(&shape_group) {
            let generated = generate_sdf(&shape_group);
            let shader_source = format!("{}\n{}", SHADER_SOURCE, generated);
            let mut shaders = main_world.resource_mut::<Assets<Shader>>();
            let handle = shaders.add(Shader::from_wgsl(shader_source));
            shader_cache.insert((*shape_group).clone(), handle);
        }
    }
}

#[derive(Resource)]
struct TracingPipeline {
    camera_layout: BindGroupLayout,
    shapes_layout: BindGroupLayout,
    first_stage_layout: BindGroupLayout,
    mid_stage_layout: BindGroupLayout,
    last_stage_layout: BindGroupLayout,
}

impl FromWorld for TracingPipeline {
    fn from_world(world: &mut bevy::prelude::World) -> Self {
        let camera_bind_group_layout = world.resource::<CameraBindGroupLayout>();
        let shapes_bind_group_layout = world.resource::<ShapesBindGroupLayout>();
        let stage_bind_group_layouts = world.resource::<StageBindGroupLayouts>();
        Self {
            camera_layout: (*camera_bind_group_layout).clone(),
            shapes_layout: (*shapes_bind_group_layout).clone(),
            first_stage_layout: stage_bind_group_layouts.first.clone(),
            mid_stage_layout: stage_bind_group_layouts.mid.clone(),
            last_stage_layout: stage_bind_group_layouts.last.clone(),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
struct TracingPipelineKey {
    handle: Handle<Shader>,
    variant: TracingPipelineVariant,
}

#[derive(Clone, PartialEq, Eq, Hash)]
enum TracingPipelineVariant {
    First,
    Mid,
    Last {
        lighting: bool,
        ambient_occlusion: bool,
        iterations: bool,
    },
}

impl SpecializedRenderPipeline for TracingPipeline {
    type Key = TracingPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let mut layout = vec![self.camera_layout.clone(), self.shapes_layout.clone()];
        let mut shader_defs = vec![
            ShaderDefVal::Int("MAX_PLANES".into(), MAX_PLANES as i32),
            ShaderDefVal::Int("MAX_SPHERES".into(), MAX_SPHERES as i32),
            ShaderDefVal::Int("MAX_CUBES".into(), MAX_CUBES as i32),
        ];

        let (label, format) = match key.variant {
            TracingPipelineVariant::First => {
                layout.push(self.first_stage_layout.clone());
                shader_defs.push("FIRST_STAGE".into());
                ("first_tracing_pipeline", TextureFormat::R32Float)
            }
            TracingPipelineVariant::Mid => {
                layout.push(self.mid_stage_layout.clone());
                ("mid_tracing_pipeline", TextureFormat::R32Float)
            }
            TracingPipelineVariant::Last {
                lighting,
                ambient_occlusion,
                iterations,
            } => {
                layout.push(self.last_stage_layout.clone());
                shader_defs.push("LAST_STAGE".into());
                if lighting {
                    shader_defs.push("LIGHTING".into());
                }
                if ambient_occlusion {
                    shader_defs.push("AMBIENT_OCCLUSION".into());
                }
                if iterations {
                    shader_defs.push("ITERATIONS".into());
                }
                ("last_tracing_pipeline", TextureFormat::Rgba8Unorm)
            }
        };

        RenderPipelineDescriptor {
            label: Some(label.into()),
            layout,
            push_constant_ranges: vec![],
            vertex: fullscreen_shader_vertex_state(),
            fragment: Some(FragmentState {
                shader: key.handle,
                shader_defs,
                entry_point: "main".into(),
                targets: vec![Some(ColorTargetState {
                    format,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: default(),
            multisample: default(),
            depth_stencil: None,
        }
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
    shape_group: Res<ShapeGroup>,
    shader_cache: Res<ShaderCache>,
    pipeline_cache: Res<PipelineCache>,
    pipeline: Res<TracingPipeline>,
    mut specialized_pipeline: ResMut<SpecializedRenderPipelines<TracingPipeline>>,
) {
    let handle = shader_cache.get(&shape_group);
    if let Some(handle) = handle {
        pipelines.invalid = false;
        pipelines.first_id = specialized_pipeline.specialize(
            &pipeline_cache,
            &pipeline,
            TracingPipelineKey {
                handle: handle.clone(),
                variant: TracingPipelineVariant::First,
            },
        );
        pipelines.mid_id = specialized_pipeline.specialize(
            &pipeline_cache,
            &pipeline,
            TracingPipelineKey {
                handle: handle.clone(),
                variant: TracingPipelineVariant::Mid,
            },
        );
        pipelines.last_id = specialized_pipeline.specialize(
            &pipeline_cache,
            &pipeline,
            TracingPipelineKey {
                handle: handle.clone(),
                variant: TracingPipelineVariant::Last {
                    lighting: true,
                    ambient_occlusion: true,
                    iterations: false,
                },
            },
        );
    } else {
        pipelines.invalid = true;
        pipelines.first_id = CachedRenderPipelineId::INVALID;
        pipelines.mid_id = CachedRenderPipelineId::INVALID;
        pipelines.last_id = CachedRenderPipelineId::INVALID;
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
