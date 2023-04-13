mod camera;
mod node;
mod shaders;
mod shape;
mod stages;
mod tracing_pipelines;
mod upsampling_pipeline;

pub use self::shape::{Shape, ShapeType};
use self::{
    camera::CameraPlugin, node::RayMarchingNode, shape::ShapePlugin, stages::StagesPlugin,
    tracing_pipelines::TracingPlugin, upsampling_pipeline::UpsamplingPipeline,
};
use bevy::{
    core_pipeline::core_3d,
    prelude::*,
    render::{render_graph::RenderGraph, RenderApp, RenderSet},
};

#[derive(Default)]
pub struct RayMarchingPlugin;

impl Plugin for RayMarchingPlugin {
    fn build(&self, app: &mut App) {
        shaders::load_shaders(app);

        app.add_plugin(CameraPlugin)
            .add_plugin(ShapePlugin)
            .add_plugin(StagesPlugin)
            .add_plugin(TracingPlugin);

        let render_app = &mut app.sub_app_mut(RenderApp);
        render_app.init_resource::<UpsamplingPipeline>();

        let world = &mut render_app.world;
        let node = RayMarchingNode::new(world);

        let graph_3d = world
            .resource_mut::<RenderGraph>()
            .into_inner()
            .get_sub_graph_mut(core_3d::graph::NAME)
            .expect("Error");

        graph_3d.add_node(RayMarchingNode::NAME, node);
        graph_3d.add_slot_edge(
            graph_3d.input_node().id,
            core_3d::graph::input::VIEW_ENTITY,
            RayMarchingNode::NAME,
            RayMarchingNode::IN_VIEW,
        );
        graph_3d.add_node_edge(core_3d::graph::node::MAIN_PASS, RayMarchingNode::NAME);
    }
}

/*

extracted data:
    shape
    camera

uniform data:
    shapes (global)
    camera (per view)
    target view sizes (per view, per target)

textures:
    per view

bind group layout:
    shapes
    camera
    target view (texture, size)

pipeline:
    ray marching (shapes, camera, target view)
    post processing

bind group




tps: target pixel size
pt: previous texture


first -> inter -> inter -> inter -> last
tps      tps      tps      tps      tps
         pt       pt       pt       pt





*/
