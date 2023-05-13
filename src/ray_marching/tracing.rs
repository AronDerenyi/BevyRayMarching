use super::{
    environment::EnvironmentBindGroupLayout,
    shape::{ShapeGroup, ShapesBindGroupLayout, MAX_CUBES, MAX_PLANES, MAX_SPHERES, MAX_IMAGES},
    stages::StageBindGroupLayouts,
    view::ViewBindGroupLayout,
    RayMarching,
};
use crate::ray_marching::shape::Operation::{self, Intersection, Union};
use bevy::{
    core_pipeline::fullscreen_vertex_shader::fullscreen_shader_vertex_state,
    prelude::{
        default, Assets, Commands, Component, Entity, FromWorld, Handle, IntoSystemAppConfig,
        IntoSystemConfig, Plugin, Query, Res, ResMut, Resource,
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
            let sdf = generate_sdf(&shape_group);
            let material = generate_material(&shape_group);
            println!("{sdf}");
            println!("{material}");
            let shader_source = format!("{}\n{}\n{}", SHADER_SOURCE, sdf, material);
            let mut shaders = main_world.resource_mut::<Assets<Shader>>();
            let handle = shaders.add(Shader::from_wgsl(shader_source));
            shader_cache.insert((*shape_group).clone(), handle);
        }
    }
}

#[derive(Resource)]
struct TracingPipeline {
    view_layout: BindGroupLayout,
    shapes_layout: BindGroupLayout,
    environment_layout: BindGroupLayout,
    first_stage_layout: BindGroupLayout,
    mid_stage_layout: BindGroupLayout,
    last_stage_layout: BindGroupLayout,
}

impl FromWorld for TracingPipeline {
    fn from_world(world: &mut bevy::prelude::World) -> Self {
        let view_bind_group_layout = world.resource::<ViewBindGroupLayout>();
        let shapes_bind_group_layout = world.resource::<ShapesBindGroupLayout>();
        let environment_bind_group_layout = world.resource::<EnvironmentBindGroupLayout>();
        let stage_bind_group_layouts = world.resource::<StageBindGroupLayouts>();
        Self {
            view_layout: (*view_bind_group_layout).clone(),
            shapes_layout: (*shapes_bind_group_layout).clone(),
            environment_layout: (*environment_bind_group_layout).clone(),
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
    First {
        iterations: u32,
    },
    Mid {
        iterations: u32,
    },
    Last {
        materials: bool,
        lighting: bool,
        ambient_occlusion: bool,
        shadow: bool,
        debug_iterations: bool,
        debug_sdf: bool,
    },
}

impl SpecializedRenderPipeline for TracingPipeline {
    type Key = TracingPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let mut layout = vec![self.view_layout.clone(), self.shapes_layout.clone()];
        let mut shader_defs = vec![
            ShaderDefVal::Int("MAX_PLANES".into(), MAX_PLANES as i32),
            ShaderDefVal::Int("MAX_SPHERES".into(), MAX_SPHERES as i32),
            ShaderDefVal::Int("MAX_CUBES".into(), MAX_CUBES as i32),
            ShaderDefVal::Int("MAX_IMAGES".into(), MAX_IMAGES as i32),
            ShaderDefVal::Int("FAR".into(), 64),
        ];

        let (label, format) = match key.variant {
            TracingPipelineVariant::First { iterations } => {
                layout.push(self.first_stage_layout.clone());
                shader_defs.push("FIRST_STAGE".into());
                shader_defs.push(ShaderDefVal::UInt("ITERATIONS".into(), iterations));
                ("first_tracing_pipeline", TextureFormat::R32Float)
            }
            TracingPipelineVariant::Mid { iterations } => {
                layout.push(self.mid_stage_layout.clone());
                shader_defs.push(ShaderDefVal::UInt("ITERATIONS".into(), iterations));
                ("mid_tracing_pipeline", TextureFormat::R32Float)
            }
            TracingPipelineVariant::Last {
                materials,
                lighting,
                ambient_occlusion,
                shadow,
                debug_iterations,
                debug_sdf,
            } => {
                layout.push(self.environment_layout.clone());
                layout.push(self.last_stage_layout.clone());
                shader_defs.push("LAST_STAGE".into());
                if materials {
                    shader_defs.push("MATERIALS".into());
                }
                if lighting {
                    shader_defs.push("LIGHTING".into());
                }
                if ambient_occlusion {
                    shader_defs.push("AMBIENT_OCCLUSION".into());
                }
                if shadow {
                    shader_defs.push("SHADOW".into());
                }
                if debug_iterations {
                    shader_defs.push("DEBUG_ITERATIONS".into());
                }
                if debug_sdf {
                    shader_defs.push("DEBUG_SDF".into());
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

#[derive(Component)]
pub struct TracingPipelines {
    pub first_id: CachedRenderPipelineId,
    pub mid_id: CachedRenderPipelineId,
    pub last_id: CachedRenderPipelineId,
}

fn queue_pipelines(
    mut commands: Commands,
    entities: Query<(Entity, &RayMarching)>,
    shape_group: Res<ShapeGroup>,
    shader_cache: Res<ShaderCache>,
    pipeline_cache: Res<PipelineCache>,
    pipeline: Res<TracingPipeline>,
    mut specialized_pipeline: ResMut<SpecializedRenderPipelines<TracingPipeline>>,
) {
    let handle = shader_cache.get(&shape_group);
    if let Some(handle) = handle {
        let entities = entities
            .iter()
            .map(|(entity, ray_marching)| {
                (
                    entity,
                    TracingPipelines {
                        first_id: specialized_pipeline.specialize(
                            &pipeline_cache,
                            &pipeline,
                            TracingPipelineKey {
                                handle: handle.clone(),
                                variant: TracingPipelineVariant::First {
                                    iterations: ray_marching.iterations,
                                },
                            },
                        ),
                        mid_id: specialized_pipeline.specialize(
                            &pipeline_cache,
                            &pipeline,
                            TracingPipelineKey {
                                handle: handle.clone(),
                                variant: TracingPipelineVariant::Mid {
                                    iterations: ray_marching.iterations,
                                },
                            },
                        ),
                        last_id: specialized_pipeline.specialize(
                            &pipeline_cache,
                            &pipeline,
                            TracingPipelineKey {
                                handle: handle.clone(),
                                variant: TracingPipelineVariant::Last {
                                    materials: ray_marching.materials,
                                    lighting: ray_marching.lighting,
                                    ambient_occlusion: ray_marching.ambient_occlusion,
                                    shadow: ray_marching.shadow,
                                    debug_iterations: ray_marching.debug_iterations,
                                    debug_sdf: ray_marching.debug_sdf,
                                },
                            },
                        ),
                    },
                )
            })
            .collect::<Vec<_>>();

        commands.insert_or_spawn_batch(entities);
    }
}

fn generate_sdf(group: &ShapeGroup) -> String {
    let mut group_index = 0u8;
    format!(
        "fn sdf_generated(pnt: vec3<f32>) -> f32 {{\n{}return dist_0;\n}}",
        generate_group_sdf(group, &mut group_index, false)
    )
}

fn generate_material(group: &ShapeGroup) -> String {
    let mut group_index = 0u8;
    format!(
        "fn sdf_material_generated(pnt: vec3<f32>) -> SDFMaterialResult {{\n{}return SDFMaterialResult(dist_0, material_0);\n}}",
        generate_group_sdf(group, &mut group_index, true)
    )
}

fn generate_group_sdf(group: &ShapeGroup, index: &mut u8, material: bool) -> String {
    let group_index = *index;
    *index += 1;

    let mut source = format!(
        "var dist_{group_index} = {};\n",
        match group.operation {
            Union => "#{FAR}f",
            Intersection => "-#{FAR}f",
        }
    );
    if material {
        source += &format!("var material_{group_index} = Material(vec3(1.0));\n");
    }
    source += &generate_shapes_sdf(
        group_index,
        group.operation,
        "plane",
        &group.plane_index_range,
        material,
    );
    source += &generate_shapes_sdf(
        group_index,
        group.operation,
        "sphere",
        &group.sphere_index_range,
        material,
    );
    source += &generate_shapes_sdf(
        group_index,
        group.operation,
        "cube",
        &group.cube_index_range,
        material,
    );
    source += &generate_shapes_sdf(
        group_index,
        group.operation,
        "image",
        &group.image_index_range,
        material,
    );

    for child in group.children.iter() {
        let child_index = *index;
        let child_source = generate_group_sdf(child, index, material);
        source += &child_source;
        source += &generate_operation(
            group.operation,
            group_index,
            format!(
                "{}dist_{child_index}",
                if child.negative { "-" } else { "" }
            ),
            if material {
                Some(format!("material_{child_index}"))
            } else {
                None
            },
        );
    }

    source
}

fn generate_shapes_sdf(
    index: u8,
    operation: Operation,
    shape: &str,
    index_range: &Range<u8>,
    material: bool,
) -> String {
    match index_range.len() {
        0 => String::new(),
        1 => generate_operation(
            operation,
            index,
            format!("sdf_{shape}({}u, pnt)", index_range.start),
            if material {
                Some(format!("shapes.{shape}s[{}u].material", index_range.start))
            } else {
                None
            },
        ),
        _ => generate_for_loop(
            index_range,
            generate_operation(
                operation,
                index,
                format!("sdf_{shape}(i, pnt)"),
                if material {
                    Some(format!("shapes.{shape}s[i].material"))
                } else {
                    None
                },
            ),
        ),
    }
}

fn generate_for_loop(range: &Range<u8>, inner: String) -> String {
    format!(
        "for (var i = {}u; i < {}u; i = i + 1u) {{\n{}}}\n",
        range.start, range.end, inner
    )
}

fn generate_operation(
    operation: Operation,
    index: u8,
    dist: String,
    material: Option<String>,
) -> String {
    match material {
        None => match operation {
            Union => format!("dist_{index} = min(dist_{index}, {dist});\n"),
            Intersection => format!("dist_{index} = max(dist_{index}, {dist});\n"),
        },
        Some(material) => match operation {
            Union => format!(
                "if min_select(&dist_{index}, {dist}) {{ material_{index} = {material}; }}\n"
            ),
            Intersection => format!(
                "if max_select(&dist_{index}, {dist}) {{ material_{index} = {material}; }}\n"
            ),
        },
    }
}
