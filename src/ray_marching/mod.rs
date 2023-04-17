mod node;
mod shape;
mod stages;
mod tracing;
mod upsampling;
mod view;

pub use self::shape::{Material, Operation, Primitive, Shape, ShapeType};
use self::{
    node::RayMarchingNode, shape::ShapePlugin, stages::StagesPlugin, tracing::TracingPlugin,
    upsampling::UpsamplingPlugin, view::ViewPlugin,
};
use bevy::{
    core_pipeline::core_3d,
    ecs::query::QueryItem,
    prelude::*,
    render::{
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        render_graph::RenderGraph,
        RenderApp,
    },
};

#[derive(Default)]
pub struct RayMarchingPlugin;

impl Plugin for RayMarchingPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(ExtractComponentPlugin::<RayMarching>::default())
            .add_plugin(ViewPlugin)
            .add_plugin(ShapePlugin)
            .add_plugin(StagesPlugin)
            .add_plugin(TracingPlugin)
            .add_plugin(UpsamplingPlugin);

        let render_app = &mut app.sub_app_mut(RenderApp);
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

#[derive(Component, Clone)]
pub struct RayMarching {
    pub resolution_start: u32,
    pub resolution_scaling: u32,
    pub resolution_scale: f32,

    pub iterations: u32,

    pub materials: bool,
    pub lighting: bool,
    pub ambient_occlusion: bool,
    pub debug_iterations: bool,
}

impl Default for RayMarching {
    fn default() -> Self {
        Self {
            resolution_start: 16,
            resolution_scaling: 2,
            resolution_scale: 0.5,
            iterations: 8,
            materials: true,
            lighting: true,
            ambient_occlusion: true,
            debug_iterations: false,
        }
    }
}

impl ExtractComponent for RayMarching {
    type Query = &'static Self;
    type Filter = With<Camera>;
    type Out = Self;

    fn extract_component(ray_marching: QueryItem<'_, Self::Query>) -> Option<Self::Out> {
        Some(ray_marching.clone())
    }
}
