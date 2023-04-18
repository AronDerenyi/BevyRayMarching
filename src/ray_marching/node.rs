use super::{
    environment::{EnvironmentBindGroup, EnvironmentUniformIndex},
    shape::ShapesBindGroup,
    stages::{StageBindGroups, StageTextures, StageUniformIndices},
    tracing::TracingPipelines,
    upsampling::UpsamplingPipeline,
    view::{ViewBindGroup, ViewUniformIndex},
};
use bevy::{
    prelude::*,
    render::{
        camera::ExtractedCamera,
        render_graph::{Node, SlotInfo, SlotType},
        render_resource::*,
        view::ViewTarget,
    },
};

pub(super) struct RayMarchingNode {
    view_query: QueryState<(
        &'static ExtractedCamera,
        &'static ViewTarget,
        &'static ViewUniformIndex,
        &'static EnvironmentUniformIndex,
        &'static StageTextures,
        &'static StageUniformIndices,
        &'static StageBindGroups,
        &'static TracingPipelines,
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
        let pipeline_cache = world.resource::<PipelineCache>();
        let upsampling_pipeline = world.resource::<UpsamplingPipeline>();
        let view_entity = graph.get_input_entity(Self::IN_VIEW)?;

        let Ok((
            camera,
            target,
            view_index,
            environment_index,
            stage_textures,
            stage_indices,
            stage_bind_groups,
            tracing_pipelines,
        )) = self.view_query.get_manual(world, view_entity) else {
            return Ok(());
        };

        let (
            Some(first_tracing_pipeline),
            Some(mid_tracing_pipeline),
            Some(last_tracing_pipeline),
            Some(upsampling_pipeline),
        ) = (
            pipeline_cache.get_render_pipeline(tracing_pipelines.first_id),
            pipeline_cache.get_render_pipeline(tracing_pipelines.mid_id),
            pipeline_cache.get_render_pipeline(tracing_pipelines.last_id),
            pipeline_cache.get_render_pipeline(upsampling_pipeline.id),
        ) else {
            return Ok(());
        };

        let view_bind_group = world.resource::<ViewBindGroup>();
        let shapes_bind_group = world.resource::<ShapesBindGroup>();
        let environment_bind_group = world.resource::<EnvironmentBindGroup>();

        {
            let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
                label: Some("first_tracing_render_pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &stage_textures.first.default_view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Load,
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });

            render_pass.set_render_pipeline(first_tracing_pipeline);
            render_pass.set_bind_group(0, view_bind_group, &[view_index.index()]);
            render_pass.set_bind_group(1, shapes_bind_group, &[]);
            render_pass.set_bind_group(2, &stage_bind_groups.first, &[stage_indices.first]);
            render_pass.draw(0..3, 0..1);
        }

        for (index, texture) in stage_textures.mid.iter().enumerate() {
            let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
                label: Some("mid_tracing_render_pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &texture.default_view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Load,
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });

            render_pass.set_render_pipeline(mid_tracing_pipeline);
            render_pass.set_bind_group(0, view_bind_group, &[view_index.index()]);
            render_pass.set_bind_group(1, shapes_bind_group, &[]);
            render_pass.set_bind_group(
                2,
                &stage_bind_groups.mid[index],
                &[stage_indices.mid[index]],
            );
            render_pass.draw(0..3, 0..1);
        }

        {
            let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
                label: Some("last_tracing_render_pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &stage_textures.last.default_view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Load,
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });

            render_pass.set_render_pipeline(last_tracing_pipeline);
            render_pass.set_bind_group(0, view_bind_group, &[view_index.index()]);
            render_pass.set_bind_group(1, shapes_bind_group, &[]);
            render_pass.set_bind_group(2, environment_bind_group, &[environment_index.index()]);
            render_pass.set_bind_group(3, &stage_bind_groups.last, &[stage_indices.last]);
            render_pass.draw(0..3, 0..1);
        }

        {
            let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
                label: Some("upsampling_render_pass"),
                color_attachments: &[Some(target.get_unsampled_color_attachment(Operations {
                    load: LoadOp::Load,
                    store: true,
                }))],
                depth_stencil_attachment: None,
            });

            if let Some(viewport) = camera.viewport.as_ref() {
                render_pass.set_camera_viewport(viewport);
            }

            render_pass.set_render_pipeline(upsampling_pipeline);
            render_pass.set_bind_group(0, &stage_bind_groups.upsampling, &[]);
            render_pass.draw(0..3, 0..1);
        }

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
