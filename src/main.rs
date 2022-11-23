pub mod ray_marching;

use bevy::prelude::*;
use ray_marching::RayMarchingPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(RayMarchingPlugin)
        .run();
}
