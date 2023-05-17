use bevy::{
    asset::{AssetLoader, Error, LoadContext, LoadedAsset},
    prelude::{AddAsset, Plugin, Vec3},
    render::render_resource::Extent3d,
};

use crate::model::Model;

use super::ShapeImage;

pub struct ShapeLoaderPlugin;

impl Plugin for ShapeLoaderPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.init_asset_loader::<PLYLoader>();
        app.init_asset_loader::<SDFLoader>();
    }
}

#[derive(Default)]
struct PLYLoader;

impl AssetLoader for PLYLoader {
    fn extensions(&self) -> &[&str] {
        &["ply"]
    }

    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> bevy::utils::BoxedFuture<'a, Result<(), Error>> {
        Box::pin(async move {
            let string = String::from(std::str::from_utf8(bytes)?);
            let shape_image = Model::from_ply(string)?.to_shape_image(
                Extent3d {
                    width: 64,
                    height: 64,
                    depth_or_array_layers: 64,
                },
                4,
            );
            let asset = LoadedAsset::new(shape_image);
            load_context.set_default_asset(asset);
            Ok(())
        })
    }
}

#[derive(Default)]
struct SDFLoader;

impl AssetLoader for SDFLoader {
    fn extensions(&self) -> &[&str] {
        &["sdf"]
    }

    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> bevy::utils::BoxedFuture<'a, Result<(), Error>> {
        Box::pin(async move {
            let asset = LoadedAsset::new(ShapeImage::from(bytes));
            load_context.set_default_asset(asset);
            Ok(())
        })
    }
}

impl From<ShapeImage> for Box<[u8]> {
    fn from(shape: ShapeImage) -> Self {
        let mut bytes = Vec::<u8>::with_capacity(123);
        
        bytes.extend_from_slice(&shape.size.x.to_le_bytes());
        bytes.extend_from_slice(&shape.size.y.to_le_bytes());
        bytes.extend_from_slice(&shape.size.z.to_le_bytes());
        bytes.extend_from_slice(&shape.resolution.width.to_le_bytes());
        bytes.extend_from_slice(&shape.resolution.height.to_le_bytes());
        bytes.extend_from_slice(&shape.resolution.depth_or_array_layers.to_le_bytes());
        for data in shape.data {
            bytes.extend_from_slice(&data.to_le_bytes());
        }
        
        bytes.into()
    }
}

impl From<&[u8]> for ShapeImage {
    fn from(bytes: &[u8]) -> Self {
        Self {
            size: Vec3 {
                x: f32::from_le_bytes(bytes[0..4].try_into().unwrap()),
                y: f32::from_le_bytes(bytes[4..8].try_into().unwrap()),
                z: f32::from_le_bytes(bytes[8..12].try_into().unwrap()),
            },
            resolution: Extent3d {
                width: u32::from_le_bytes(bytes[12..16].try_into().unwrap()),
                height: u32::from_le_bytes(bytes[16..20].try_into().unwrap()),
                depth_or_array_layers: u32::from_le_bytes(bytes[20..24].try_into().unwrap()),
            },
            data: bytes[24..]
                    .chunks(4)
                    .map(|chunk| f32::from_le_bytes(chunk.try_into().unwrap()))
                    .collect(),
        }
    }
}
