mod ray_marching;
mod user_interface;
use bevy::diagnostic::{Diagnostics, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;
use bevy::{diagnostic::LogDiagnosticsPlugin, input::mouse::MouseWheel};
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use ray_marching::RayMarching;
use ray_marching::{
    Material, RayMarchingPlugin, Shape,
    ShapeType::{Cube, Intersection, Plane, Sphere},
};
use std::f32::consts;
use user_interface::UIPlugin;

/*
LogPlugin {
filter: "wgpu_core".into(),
level: Level::INFO,
}
*/

#[derive(Component)]
struct OrbitControls {
    pivot: Vec3,
    rotation: Vec2,
    zoom: f32,
}

#[derive(Component)]
struct Bouncing;

fn main() {
    //    WindowPlugin {
    //        window: WindowDescriptor {
    //            title: "Ray marching".to_string(),
    //            present_mode: PresentMode::Immediate,
    //            width: 960.0,
    //            height: 640.0,
    //            ..default()
    //        },
    //        ..default()
    //    }
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(LogDiagnosticsPlugin::default())
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_plugin(EguiPlugin)
        .add_plugin(RayMarchingPlugin)
        .add_plugin(UIPlugin)
        .add_startup_system(setup)
        .add_system(orbit_controller)
        .add_system(orbit_updater)
        .add_system(bouncing_updater)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn((
        Camera3dBundle {
            projection: Projection::Perspective(PerspectiveProjection { ..default() }),
            camera: Camera {
                //                viewport: Some(Viewport {
                //                    physical_position: UVec2::new(0, 0),
                //                    physical_size: UVec2::new(300, 300),
                //                    ..default()
                //                }),
                ..default()
            },
            ..default()
        },
        RayMarching {
            lighting: true,
            ambient_occlusion: true,
            debug_iterations: false,
            ..default()
        },
        OrbitControls {
            pivot: Vec3::ZERO,
            rotation: Vec2::ZERO,
            zoom: 10.0,
        },
    ));

    commands
        .spawn((
            Name::new("Root"),
            Shape {
                shape_type: Intersection,
                ..default()
            },
            Transform::default(),
            GlobalTransform::default(),
        ))
        .with_children(|builder| {
            builder.spawn((
                Name::new("Sphere"),
                Shape {
                    shape_type: Sphere { radius: 1.3 },
                    ..default()
                },
                Material {
                    color: Vec3::new(1.0, 0.0, 0.0),
                },
                Transform::default(),
                GlobalTransform::default(),
            ));
            builder.spawn((
                Name::new("ClipCube"),
                Shape {
                    shape_type: Cube {
                        size: Vec3::new(1.0, 1.0, 1.0),
                    },
                    ..default()
                },
                Material {
                    color: Vec3::new(0.0, 1.0, 0.0),
                },
                Transform::default(),
                GlobalTransform::default(),
            ));
            builder.spawn((
                Name::new("HoleCubeX"),
                Shape {
                    shape_type: Cube {
                        size: Vec3::new(1.1, 0.4, 0.4),
                    },
                    negative: true,
                },
                Material {
                    color: Vec3::new(1.0, 0.0, 1.0),
                },
                Transform::default(),
                GlobalTransform::default(),
            ));
            builder.spawn((
                Name::new("HoleCubeY"),
                Shape {
                    shape_type: Cube {
                        size: Vec3::new(0.4, 1.1, 0.4),
                    },
                    negative: true,
                },
                Material {
                    color: Vec3::new(0.0, 1.0, 1.0),
                },
                Transform::default(),
                GlobalTransform::default(),
            ));
            builder.spawn((
                Name::new("HoleCubeZ"),
                Shape {
                    shape_type: Cube {
                        size: Vec3::new(0.4, 0.4, 1.1),
                    },
                    negative: true,
                },
                Material {
                    color: Vec3::new(0.0, 0.0, 1.0),
                },
                Transform::default(),
                GlobalTransform::default(),
            ));
        });
}

fn orbit_controller(
    mut orbits: Query<&mut OrbitControls>,
    mut scroll_event: EventReader<MouseWheel>,
) {
    for event in scroll_event.iter() {
        for mut orbit in orbits.iter_mut() {
            orbit.rotation -= Vec2::new(event.x, event.y) * 0.0025;
            if orbit.rotation.y < 0.0 {
                orbit.rotation.y = 0.0;
            }
            if orbit.rotation.y > consts::PI {
                orbit.rotation.y = consts::PI;
            }
        }
    }
}

fn orbit_updater(mut orbits: Query<(&mut Transform, &OrbitControls)>) {
    for (mut transform, orbit) in orbits.iter_mut() {
        let rotation =
            Quat::from_rotation_z(orbit.rotation.x) * Quat::from_rotation_x(orbit.rotation.y);
        transform.translation = orbit.pivot + rotation * Vec3::Z * orbit.zoom;
        transform.rotation = rotation;
    }
}

fn bouncing_updater(mut transforms: Query<&mut Transform, With<Bouncing>>, time: Res<Time>) {
    for mut transform in transforms.iter_mut() {
        transform.translation.z = time.elapsed_seconds().sin() * 0.5 + 0.05;
    }
}
