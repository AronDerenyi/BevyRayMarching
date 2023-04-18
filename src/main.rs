mod ray_marching;
mod user_interface;
use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
use bevy::prelude::*;
use bevy::{diagnostic::LogDiagnosticsPlugin, input::mouse::MouseWheel};
use bevy_egui::EguiPlugin;
use ray_marching::RayMarching;
use ray_marching::{
    Environment, Material,
    Operation::Intersection,
    Primitive::{Cube, Sphere},
    RayMarchingPlugin, Shape,
    ShapeType::{Compound, Primitive},
};
use std::f32::consts;
use user_interface::UIPlugin;

#[derive(Component)]
struct OrbitControls {
    pivot: Vec3,
    rotation: Vec2,
    zoom: f32,
}

#[derive(Component)]
struct Bouncing;

fn main() {
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
        Camera3dBundle::default(),
        RayMarching {
            lighting: true,
            ambient_occlusion: true,
            debug_iterations: false,
            ..default()
        },
        Environment {
            sky: Vec3::new(0.5, 0.8, 1.0),
            sun_direction: Vec3::new(0.5, 0.8, 1.0).normalize(),
            sun_light: Vec3::new(1.0, 0.8, 0.6),
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
            Shape::default(),
            Transform::default(),
            GlobalTransform::default(),
        ))
        .with_children(|builder| {
            for y in -2..=2 {
                for x in -2..=2 {
                    builder.spawn((
                        Name::new(format!("Sphere_{x}_{y}")),
                        Shape {
                            shape_type: Primitive(
                                Sphere { radius: 0.4 },
                                Material {
                                    color: Vec3::new(1.0, 1.0, 1.0),
                                },
                            ),
                            ..default()
                        },
                        Transform::from_xyz(x as f32, y as f32, 0.0),
                        GlobalTransform::default(),
                    ));
                }
            }
        });
    return;

    commands
        .spawn((
            Name::new("Root"),
            Shape {
                shape_type: Compound(Intersection),
                ..default()
            },
            Transform::default(),
            GlobalTransform::default(),
        ))
        .with_children(|builder| {
            builder.spawn((
                Name::new("Sphere"),
                Shape {
                    shape_type: Primitive(
                        Sphere { radius: 1.3 },
                        Material {
                            color: Vec3::new(0.1, 0.1, 0.1),
                        },
                    ),
                    ..default()
                },
                Transform::default(),
                GlobalTransform::default(),
            ));
            builder.spawn((
                Name::new("ClipCube"),
                Shape {
                    shape_type: Primitive(
                        Cube {
                            size: Vec3::new(1.0, 1.0, 1.0),
                        },
                        Material {
                            color: Vec3::new(1.0, 1.0, 1.0),
                        },
                    ),
                    ..default()
                },
                Transform::default(),
                GlobalTransform::default(),
            ));
            builder.spawn((
                Name::new("HoleCubeX"),
                Shape {
                    shape_type: Primitive(
                        Cube {
                            size: Vec3::new(2.0, 0.4, 0.4),
                        },
                        Material {
                            color: Vec3::new(1.0, 0.2, 0.8),
                        },
                    ),
                    negative: true,
                },
                Transform::default(),
                GlobalTransform::default(),
            ));
            builder.spawn((
                Name::new("HoleCubeY"),
                Shape {
                    shape_type: Primitive(
                        Cube {
                            size: Vec3::new(0.4, 2.0, 0.4),
                        },
                        Material {
                            color: Vec3::new(1.0, 0.2, 0.8),
                        },
                    ),
                    negative: true,
                },
                Transform::default(),
                GlobalTransform::default(),
            ));
            builder.spawn((
                Name::new("HoleCubeZ"),
                Shape {
                    shape_type: Primitive(
                        Cube {
                            size: Vec3::new(0.4, 0.4, 2.0),
                        },
                        Material {
                            color: Vec3::new(1.0, 0.2, 0.8),
                        },
                    ),
                    negative: true,
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
