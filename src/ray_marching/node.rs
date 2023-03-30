use super::{
    pipelines::{CameraIndex, CamerasMeta, Pipelines, ShapesMeta},
    Textures,
};
use bevy::{
    prelude::*,
    render::{
        camera::ExtractedCamera,
        render_graph::{Node, SlotInfo, SlotType},
        render_phase::TrackedRenderPass,
        render_resource::*,
        view::ViewTarget,
    },
};

pub(super) struct RayMarchingNode {
    view_query: QueryState<(
        &'static Textures,
        &'static ExtractedCamera,
        &'static ViewTarget,
        &'static CameraIndex,
    )>,
}

impl RayMarchingNode {
    pub(super) const NAME: &str = "ray_marching_node";
    pub(super) const IN_VIEW: &str = "view";

    pub(super) fn new(world: &mut World) -> Self {
        Self {
            view_query: QueryState::new(world),
        }
    }
}

impl Node for RayMarchingNode {
    fn run(
        &self,
        graph: &mut bevy::render::render_graph::RenderGraphContext,
        render_context: &mut bevy::render::renderer::RenderContext,
        world: &World,
    ) -> Result<(), bevy::render::render_graph::NodeRunError> {
        let view_entity = graph.get_input_entity(Self::IN_VIEW)?;
        let Ok((textures, camera, target, index)) = self.view_query.get_manual(world, view_entity) else {
            return Ok(());
        };

        let pipeline_cache = world.resource::<PipelineCache>();
        let pipelines = world.resource::<Pipelines>();
        let pipeline = pipelines.pipeline(pipeline_cache);
        let cameras = world.resource::<CamerasMeta>();
        let shapes = world.resource::<ShapesMeta>();

        //        let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
        //            label: Some("Test Pass"),
        //            color_attachments: &[Some(target.get_unsampled_color_attachment(Operations {
        //                load: LoadOp::Load,
        //                store: true,
        //            }))],
        //            depth_stencil_attachment: None,
        //        });

        {
            let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
                label: Some("Test Pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &textures.texture.default_view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Load,
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });

            //        if let Some(viewport) = camera.viewport.as_ref() {
            //            render_pass.set_camera_viewport(viewport);
            //        }

            render_pass.set_render_pipeline(pipeline);
            render_pass.set_bind_group(0, cameras.bind_group(), &[index.index()]);
            render_pass.set_bind_group(1, shapes.bind_group(), &[]);
            render_pass.draw(0..3, 0..1);
        }

        {
            let bind_group = render_context.render_device().create_bind_group(&BindGroupDescriptor {
                label: Some("filter bind group"),
                layout: &pipelines.filter_bind_layout,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::TextureView(&textures.texture.default_view),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: BindingResource::Sampler(&pipelines.sampler),
                    },
                ],
            });

            let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
                label: Some("Filter Pass"),
                color_attachments: &[Some(target.get_unsampled_color_attachment(Operations {
                    load: LoadOp::Load,
                    store: true,
                }))],
                depth_stencil_attachment: None,
            });

            if let Some(viewport) = camera.viewport.as_ref() {
                render_pass.set_camera_viewport(viewport);
            }

            render_pass.set_render_pipeline(pipelines.filter_pipeline(pipeline_cache));
            render_pass.set_bind_group(0, &bind_group, &[]);
            render_pass.draw(0..3, 0..1);
        }

        //        let mut render_pass =
        //            TrackedRenderPass::new(render_context.command_encoder.begin_render_pass(
        //                    &RenderPassDescriptor {
        //                        label: Some("Test Pass"),
        //                        color_attachments: &[Some(RenderPassColorAttachment {
        //                            view: target.main_texture(),
        //                            resolve_target: None,
        //                            ops: Operations {
        //                                load: LoadOp::Load,
        //                                store: true,
        //                            },
        //                        })],
        //                        depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
        //                            view: target.main_texture(),
        //                            depth_ops: (),
        //                            stencil_ops: ()
        //                        }),
        //                    },
        //            ));
        //
        //        if let Some(viewport) = camera.viewport.as_ref() {
        //            render_pass.set_camera_viewport(viewport);
        //        }
        //
        //        render_pass.set_render_pipeline(pipelines.write_pipeline(pipeline_cache));
        //        render_pass.draw(0..3, 0..1);

        Ok(())
    }

    fn input(&self) -> Vec<SlotInfo> {
        vec![SlotInfo::new(Self::IN_VIEW, SlotType::Entity)]
    }

    fn output(&self) -> Vec<SlotInfo> {
        Vec::new()
    }

    fn update(&mut self, world: &mut World) {
        self.view_query.update_archetypes(world);
    }
}
