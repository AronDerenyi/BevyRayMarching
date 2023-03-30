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

const SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 698782022341067255);

const STENCIL_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 698782022341067256);

pub fn load_shaders(app: &mut App) {
    load_internal_asset!(app, SHADER_HANDLE, "ray_marching.wgsl", Shader::from_wgsl);
    load_internal_asset!(app, STENCIL_SHADER_HANDLE, "stencil_shader.wgsl", Shader::from_wgsl);
}

#[derive(Resource, Debug)]
pub struct Pipelines {
    pipeline: CachedRenderPipelineId,
    write_pipeline: CachedRenderPipelineId,
    test_pipeline: CachedRenderPipelineId,
    camera_bind_layout: BindGroupLayout,
    shapes_bind_layout: BindGroupLayout,
}

impl FromWorld for Pipelines {
    fn from_world(world: &mut World) -> Self {
        let device = world.resource::<RenderDevice>();

        let camera_bind_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: "Camera bind group layout".into(),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: Some(Camera::min_size()),
                },
                count: None,
            }],
        });

        let shapes_bind_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: "Shapes bind group layout".into(),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: Some(Shapes::min_size()),
                },
                count: None,
            }],
        });

        let mut cache = world.resource_mut::<PipelineCache>();
        let pipeline = cache.queue_render_pipeline(RenderPipelineDescriptor {
            label: Some("Ray Marching pipeline".into()),
            layout: vec![camera_bind_layout.clone(), shapes_bind_layout.clone()],
            push_constant_ranges: vec![],
            vertex: fullscreen_shader_vertex_state(),
            fragment: Some(FragmentState {
                shader: SHADER_HANDLE.typed(),
                shader_defs: vec![
                    ShaderDefVal::UInt("SPHERES".into(), Shapes::SPHERES as u32),
                    ShaderDefVal::UInt("CUBES".into(), Shapes::CUBES as u32),
                ],
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

        let write_pipeline = cache.queue_render_pipeline(RenderPipelineDescriptor {
            label: Some("Write pipeline".into()),
            layout: vec![],
            push_constant_ranges: vec![],
            vertex: fullscreen_shader_vertex_state(),
            fragment: Some(FragmentState {
                shader: STENCIL_SHADER_HANDLE.typed(),
                shader_defs: default(),
                entry_point: "write".into(),
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

        let test_pipeline = cache.queue_render_pipeline(RenderPipelineDescriptor {
            label: Some("Write pipeline".into()),
            layout: vec![],
            push_constant_ranges: vec![],
            vertex: fullscreen_shader_vertex_state(),
            fragment: Some(FragmentState {
                shader: STENCIL_SHADER_HANDLE.typed(),
                shader_defs: default(),
                entry_point: "test".into(),
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
            pipeline,
            write_pipeline,
            test_pipeline,
            camera_bind_layout,
            shapes_bind_layout,
        }
    }
}

impl<'a> Pipelines {
    pub fn pipeline(&self, cache: &'a PipelineCache) -> &'a RenderPipeline {
        cache
            .get_render_pipeline(self.pipeline)
            .expect("Pipeline not set yet")
    }

    pub fn write_pipeline(&self, cache: &'a PipelineCache) -> &'a RenderPipeline {
        cache
            .get_render_pipeline(self.write_pipeline)
            .expect("Pipeline not set yet")
    }

    pub fn test_pipeline(&self, cache: &'a PipelineCache) -> &'a RenderPipeline {
        cache
            .get_render_pipeline(self.test_pipeline)
            .expect("Pipeline not set yet")
    }
}

#[derive(ShaderType, Clone, Default)]
pub(super) struct Camera {
    pub position: Vec3,
    pub right: Vec3,
    pub up: Vec3,
    pub forward: Vec3,
}

#[derive(Component)]
pub(super) struct CameraIndex(u32);

impl CameraIndex {
    pub fn index(&self) -> u32 {
        self.0
    }
}

#[derive(Resource)]
pub(super) struct CamerasMeta {
    uniforms: DynamicUniformBuffer<Camera>,
    bind_layout: BindGroupLayout,
    bind_group: Option<BindGroup>,
}

impl FromWorld for CamerasMeta {
    fn from_world(world: &mut World) -> Self {
        let pipelines = world.resource::<Pipelines>();
        Self {
            uniforms: default(),
            bind_layout: pipelines.camera_bind_layout.clone(),
            bind_group: None,
        }
    }
}

impl CamerasMeta {
    pub fn push(&mut self, camera: Camera) -> CameraIndex {
        self.uniforms.clear();
        CameraIndex(self.uniforms.push(camera))
    }

    pub fn write(&mut self, device: &RenderDevice, queue: &RenderQueue) {
        self.uniforms.write_buffer(device, queue);
        self.bind_group = Some(device.create_bind_group(&BindGroupDescriptor {
            label: "Camera bind group".into(),
            layout: &self.bind_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: self.uniforms.binding().unwrap().clone(),
            }],
        }));
    }

    pub fn bind_group(&self) -> &BindGroup {
        self.bind_group.as_ref().expect("No camera set yet")
    }
}

#[derive(ShaderType, Clone, Default)]
pub(super) struct Transform {
    pub inv_transform: Mat4,
    pub min_scale: f32,
}

#[derive(ShaderType, Clone, Default)]
pub(super) struct Shapes {
    pub plane: Transform,
    pub spheres: [Transform; Shapes::SPHERES],
    pub cubes: [Transform; Shapes::CUBES],
}

//impl bevy::render::render_resource::encase::private::ShaderType for Shapes
//        where
//            Transform: bevy::render::render_resource::encase::private::ShaderType
//                + bevy::render::render_resource::encase::private::ShaderSize,
//            [Transform; Shapes::SPHERES]:
//                bevy::render::render_resource::encase::private::ShaderType
//                    + bevy::render::render_resource::encase::private::ShaderSize,
//            [Transform; Shapes::CUBES]: bevy::render::render_resource::encase::private::ShaderType,
//        {
//            type ExtraMetadata =
//                bevy::render::render_resource::encase::private::StructMetadata<3usize>;
//            const METADATA: bevy::render::render_resource::encase::private::Metadata<
//                Self::ExtraMetadata,
//            > = {
//                let struct_alignment = bevy :: render :: render_resource :: encase :: private :: AlignmentValue :: max ([< Transform as bevy :: render :: render_resource :: encase :: private :: ShaderType > :: METADATA . alignment () , < [Transform ; Shapes :: SPHERES] as bevy :: render :: render_resource :: encase :: private :: ShaderType > :: METADATA . alignment () , < [Transform ; Shapes :: CUBES] as bevy :: render :: render_resource :: encase :: private :: ShaderType > :: METADATA . alignment ()]) ;
//                let extra = {
//                    let mut paddings = [0; 3usize];
//                    let mut offsets = [0; 3usize];
//                    let mut offset = 0;
//                    offset += < Transform as bevy :: render :: render_resource :: encase :: private :: ShaderSize > :: SHADER_SIZE . get () ;
//                    offsets [1usize] = < [Transform ; Shapes :: SPHERES] as bevy :: render :: render_resource :: encase :: private :: ShaderType > :: METADATA . alignment () . round_up (offset) ;
//                    let padding = < [Transform ; Shapes :: SPHERES] as bevy :: render :: render_resource :: encase :: private :: ShaderType > :: METADATA . alignment () . padding_needed_for (offset) ;
//                    offset += padding;
//                    paddings[0usize] = padding;
//                    offset += < [Transform ; Shapes :: SPHERES] as bevy :: render :: render_resource :: encase :: private :: ShaderSize > :: SHADER_SIZE . get () ;
//                    offsets[2usize] = <[Transform; Shapes::CUBES] as bevy::render::render_resource::encase::private::ShaderType>::METADATA.alignment().round_up(offset);
//                    let padding = < [Transform ; Shapes :: CUBES] as bevy :: render :: render_resource :: encase :: private :: ShaderType > :: METADATA . alignment () . padding_needed_for (offset) ;
//                    offset += padding;
//                    paddings[1usize] = padding;
//                    offset += < [Transform ; Shapes :: CUBES] as bevy :: render :: render_resource :: encase :: private :: ShaderSize > :: SHADER_SIZE . get () ;
//                    paddings[2usize] = struct_alignment.padding_needed_for(offset);
//                    bevy::render::render_resource::encase::private::StructMetadata {
//                        offsets,
//                        paddings,
//                    }
//                };
//                let min_size = {
//                    let mut offset = extra.offsets[3usize - 1];
//                    offset += < [Transform ; Shapes :: CUBES] as bevy :: render :: render_resource :: encase :: private :: ShaderType > :: METADATA . min_size () . get () ;
//                    bevy::render::render_resource::encase::private::SizeValue::new(
//                            struct_alignment.round_up(offset),
//                    )
//                };
//                bevy::render::render_resource::encase::private::Metadata {
//                    alignment: struct_alignment,
//                    has_uniform_min_alignment: true,
//                    min_size,
//                    extra,
//                }
//            };
//        }

impl Shapes {
    pub const SPHERES: usize = 2;
    pub const CUBES: usize = 1;
}

#[derive(Resource)]
pub(super) struct ShapesMeta {
    uniform: UniformBuffer<Shapes>,
    bind_layout: BindGroupLayout,
    bind_group: Option<BindGroup>,
}

impl FromWorld for ShapesMeta {
    fn from_world(world: &mut World) -> Self {
        let pipelines = world.resource::<Pipelines>();
        Self {
            uniform: default(),
            bind_layout: pipelines.shapes_bind_layout.clone(),
            bind_group: None,
        }
    }
}

impl ShapesMeta {
    pub fn set(&mut self, shapes: Shapes) {
        self.uniform.set(shapes);
    }

    pub fn write(&mut self, device: &RenderDevice, queue: &RenderQueue) {
        self.uniform.write_buffer(device, queue);
        self.bind_group = Some(device.create_bind_group(&BindGroupDescriptor {
            label: "Shapes bind group".into(),
            layout: &self.bind_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: self.uniform.binding().unwrap().clone(),
            }],
        }));
    }

    pub fn bind_group(&self) -> &BindGroup {
        self.bind_group.as_ref().expect("No shapes set yet")
    }
}
