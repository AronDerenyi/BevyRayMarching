mod cameras;
mod node;
mod pipelines;
pub mod shapes;

use self::{
    cameras::ExtractedCamera,
    node::RayMarchingNode,
    pipelines::{CamerasMeta, Pipelines, ShapesMeta},
    shapes::ExtractedShape,
};
use bevy::{
    core_pipeline::core_3d,
    prelude::*,
    render::{
        extract_component::ExtractComponentPlugin, render_graph::RenderGraph, RenderApp, RenderSet,
    },
};

#[derive(Default)]
pub struct RayMarchingPlugin;

impl Plugin for RayMarchingPlugin {
    fn build(&self, app: &mut App) {
        pipelines::load_shaders(app);

        app.add_plugin(ExtractComponentPlugin::<ExtractedCamera>::default())
            .add_plugin(ExtractComponentPlugin::<ExtractedShape>::default());

        let render_app = &mut app.sub_app_mut(RenderApp);
        render_app
            .init_resource::<Pipelines>()
            .init_resource::<CamerasMeta>()
            .init_resource::<ShapesMeta>()
            .add_system(cameras::prepare_cameras.in_set(RenderSet::Prepare))
            .add_system(shapes::prepare_shapes.in_set(RenderSet::Prepare));

        let world = &mut render_app.world;
        let node = RayMarchingNode::new(world);

        //        let mut graph = world.resource_mut::<RenderGraph>();
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

//fn queue_bind_group(mut commands: Commands, views: Query<(Entity, &ViewTarget)>) {
//    for (entity, target) in views.iter() {
//        commands.entity(entity).insert(bundle)
//    }
//}
