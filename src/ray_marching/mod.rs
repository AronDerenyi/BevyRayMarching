mod camera;
mod node;
mod pipelines;
mod ray_marching_pipeline;
mod shaders;
mod shape;

pub use self::shape::Shape;
use self::{
    camera::CameraPlugin, node::RayMarchingNode, pipelines::Pipelines,
    ray_marching_pipeline::RayMarchingPipeline, shape::ShapePlugin,
};
use bevy::{
    core_pipeline::core_3d,
    prelude::*,
    render::{
        render_graph::RenderGraph,
        render_resource::{
            Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
        },
        renderer::RenderDevice,
        texture::{CachedTexture, TextureCache},
        RenderApp, RenderSet,
    },
};

#[derive(Default)]
pub struct RayMarchingPlugin;

impl Plugin for RayMarchingPlugin {
    fn build(&self, app: &mut App) {
        shaders::load_shaders(app);

        app.add_plugin(CameraPlugin);
        app.add_plugin(ShapePlugin);

        let render_app = &mut app.sub_app_mut(RenderApp);
        render_app
            .init_resource::<Pipelines>()
            .init_resource::<RayMarchingPipeline>()
            .add_system(prepare_textures.in_set(RenderSet::Prepare))
            .add_system(
                ray_marching_pipeline::queue_ray_marching_pipeline.in_set(RenderSet::Queue),
            );

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
        if let Some(UVec2 {
            x: width,
            y: height,
        }) = camera.physical_viewport_size
        {
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
                texture: texture_cache.get(&render_device, descriptor),
            });
        }
    }
}

//fn queue_bind_group(mut commands: Commands, views: Query<(Entity, &ViewTarget)>) {
//    for (entity, target) in views.iter() {
//        commands.entity(entity).insert(bundle)
//    }
//}

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

*/
