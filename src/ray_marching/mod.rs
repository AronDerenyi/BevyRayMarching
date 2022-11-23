use bevy::{
    asset::load_internal_asset,
    core_pipeline::fullscreen_vertex_shader::fullscreen_shader_vertex_state,
    prelude::*,
    reflect::TypeUuid,
    render::{
        render_graph::{Node, RenderGraph},
        render_resource::*,
        renderer::{RenderDevice, RenderQueue},
        view::ExtractedWindows,
        RenderApp,
    },
    window::WindowId,
};

const SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 698782022341067255);

#[derive(Default)]
pub struct RayMarchingPlugin;

impl Plugin for RayMarchingPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        load_internal_asset!(app, SHADER_HANDLE, "ray_marching.wgsl", Shader::from_wgsl);

        let world = &mut app.sub_app_mut(RenderApp).world;

        let node = RayMarchingNode::new(world);

        let mut graph = world.resource_mut::<RenderGraph>();
        graph.add_node("test_node", node);
        graph
            .add_node_edge(bevy::render::main_graph::node::CAMERA_DRIVER, "test_node")
            .expect("Error");
    }
}

#[derive(Component, ShaderType, Clone)]
struct RayMarchingUniforms {
    value: f32,
}

struct RayMarchingNode {
    pipeline_id: CachedRenderPipelineId,
    binding: BindGroup,
}

impl RayMarchingNode {
    fn new(world: &mut World) -> Self {
        let mut pipeline_cache = world.resource_mut::<PipelineCache>();
        let pipeline_id = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
            label: Some("Test Pipeline".into()),
            layout: None,
            vertex: fullscreen_shader_vertex_state(),
            fragment: Some(FragmentState {
                shader: SHADER_HANDLE.typed(),
                shader_defs: vec![],
                entry_point: "main".into(),
                targets: vec![Some(ColorTargetState {
                    format: TextureFormat::Bgra8UnormSrgb,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: default(),
            multisample: default(),
            depth_stencil: None,
        });

        let render_device = world.resource::<RenderDevice>();
        let render_queue = world.resource::<RenderQueue>();
        let binding_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: "Test Binding Layout".into(),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: Some(RayMarchingUniforms::min_size()),
                },
                count: None,
            }],
        });

        let mut uniform_buffer = UniformBuffer::from(RayMarchingUniforms { value: 0.5 });
        uniform_buffer.write_buffer(render_device, render_queue);

        let binding = render_device.create_bind_group(&BindGroupDescriptor {
            label: "Test Binding".into(),
            layout: &binding_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.binding().unwrap().clone(),
            }],
        });

        Self {
            pipeline_id,
            binding,
        }
    }
}

impl Node for RayMarchingNode {
    fn run(
        &self,
        _graph: &mut bevy::render::render_graph::RenderGraphContext,
        render_context: &mut bevy::render::renderer::RenderContext,
        world: &World,
    ) -> Result<(), bevy::render::render_graph::NodeRunError> {
        let extracted_windows = &world.get_resource::<ExtractedWindows>().unwrap().windows;
        let extracted_window =
            if let Some(extracted_window) = extracted_windows.get(&WindowId::primary()) {
                extracted_window
            } else {
                return Ok(());
            };

        let pipeline_cache = world.resource::<PipelineCache>();
        let pipeline = pipeline_cache
            .get_render_pipeline(self.pipeline_id)
            .unwrap();

        let mut render_pass =
            render_context
                .command_encoder
                .begin_render_pass(&RenderPassDescriptor {
                    label: Some("Test Pass"),
                    color_attachments: &[Some(RenderPassColorAttachment {
                        view: extracted_window.swap_chain_texture.as_ref().unwrap(),
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Load,
                            store: true,
                        },
                    })],
                    depth_stencil_attachment: None,
                });

        render_pass.set_pipeline(pipeline);
        render_pass.set_bind_group(0, &self.binding, &[]);
        render_pass.draw(0..3, 0..1);

        Ok(())
    }

    fn input(&self) -> Vec<bevy::render::render_graph::SlotInfo> {
        Vec::new()
    }

    fn output(&self) -> Vec<bevy::render::render_graph::SlotInfo> {
        Vec::new()
    }

    fn update(&mut self, _world: &mut World) {}
}
