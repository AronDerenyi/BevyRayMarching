mod cameras;
mod node;
mod pipelines;
pub mod shapes;

use self::{
    node::RayMarchingNode,
    pipelines::{CamerasMeta, Pipelines, ShapesMeta},
    shapes::ExtractedShape, cameras::ExtractedCamera,
};
use bevy::{
    core_pipeline::core_3d,
    prelude::*,
    render::{
        extract_component::ExtractComponentPlugin,
        render_graph::RenderGraph,
        texture::{CachedTexture, TextureCache},
        view::ExtractedView,
        RenderApp, RenderSet, render_resource::{TextureDescriptor, Extent3d, TextureDimension, TextureFormat, TextureUsages}, renderer::RenderDevice,
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
            .add_system(shapes::prepare_shapes.in_set(RenderSet::Prepare))
            .add_system(prepare_textures.in_set(RenderSet::Prepare));

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

#[derive(Component)]
struct Textures {
    texture: CachedTexture,
}

fn prepare_textures(
    mut commands: Commands,
    mut texture_cache: ResMut<TextureCache>,
    render_device: Res<RenderDevice>,
    cameras: Query<(Entity, &bevy::render::camera::ExtractedCamera)>,
) {
    for (entity, camera) in &cameras {
        if let Some(UVec2 { x: width, y: height }) = camera.physical_viewport_size {
            let descriptor = TextureDescriptor {
                label: Some("Downsample texture"),
                size: Extent3d {
                    width: width / 4,
                    height: height / 4,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Rgba8Unorm,
                usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            };
            commands.entity(entity).insert(Textures {
                texture: texture_cache.get(&render_device, descriptor)
            });
        }
    }
}

//fn queue_bind_group(mut commands: Commands, views: Query<(Entity, &ViewTarget)>) {
//    for (entity, target) in views.iter() {
//        commands.entity(entity).insert(bundle)
//    }
//}