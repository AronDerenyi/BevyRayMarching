use super::pipelines::{Camera, CamerasMeta};
use bevy::{
    ecs::query::QueryItem,
    prelude::{
        Commands, Component, Entity, GlobalTransform, Projection, Projection::Perspective, Query,
        Res, ResMut, Vec3,
    },
    render::{
        extract_component::ExtractComponent,
        renderer::{RenderDevice, RenderQueue},
    },
};

#[derive(Component, Clone)]
pub(super) struct ExtractedCamera {
    projection: Projection,
    transform: GlobalTransform,
}

impl ExtractComponent for ExtractedCamera {
    type Query = (&'static Projection, &'static GlobalTransform);
    type Filter = ();
    type Out = Self;

    fn extract_component((projection, transform): QueryItem<'_, Self::Query>) -> Option<Self> {
        Some(Self {
            projection: projection.clone(),
            transform: transform.clone(),
        })
    }
}

pub(super) fn prepare_cameras(
    mut commands: Commands,
    shapes: Query<(Entity, &ExtractedCamera)>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    mut cameras_meta: ResMut<CamerasMeta>,
) {
    for (
        entity,
        ExtractedCamera {
            projection,
            transform,
        },
    ) in shapes.iter()
    {
        let gpu_camera = match &projection {
            Perspective(projection) => {
                let tan_fov = (projection.fov * 0.5).tan();
                let matrix = transform.affine().matrix3;
                Camera {
                    position: transform.translation(),
                    right: matrix * Vec3::X * tan_fov * projection.aspect_ratio,
                    up: matrix * Vec3::Y * tan_fov,
                    forward: matrix * Vec3::NEG_Z,
                }
            }
            _ => panic!("Unsupported projection"),
        };
        commands
            .get_or_spawn(entity)
            .insert(cameras_meta.push(gpu_camera));
    }
    cameras_meta.write(&*device, &*queue);
}
