pub mod ray_marching;

use bevy::diagnostic::{Diagnostics, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;
use bevy::{diagnostic::LogDiagnosticsPlugin, input::mouse::MouseWheel};
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use ray_marching::{
    RayMarchingPlugin, Shape,
    ShapeType::{Cube, Plane, Sphere},
};
use std::f32::consts;

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
        .add_startup_system(setup)
        .add_system(orbit_controller)
        .add_system(orbit_updater)
        .add_system(bouncing_updater)
        .add_system(gui)
        .run();
}

fn gui(mut egui_contexts: EguiContexts, diagnostics: Res<Diagnostics>) {
    egui::Window::new("Diagnostics").show(egui_contexts.ctx_mut(), |ui| {
        if let Some(fps) = diagnostics.get(FrameTimeDiagnosticsPlugin::FPS) {
            ui.label(format!("{:.0} fps", fps.smoothed().unwrap_or(0.0)));
        }
        if let Some(frame_time) = diagnostics.get(FrameTimeDiagnosticsPlugin::FRAME_TIME) {
            ui.label(format!("{:.2} ms", frame_time.smoothed().unwrap_or(0.0)));
        }
    });
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
        OrbitControls {
            pivot: Vec3::ZERO,
            rotation: Vec2::ZERO,
            zoom: 10.0,
        },
    ));

    commands.spawn((
        Shape {
            shape_type: Plane,
            ..default()
        },
        Transform {
            translation: Vec3::new(0.0, 0.0, -0.0),
            ..default()
        },
        GlobalTransform::IDENTITY,
    ));
    //    commands.spawn((
    //        Shape {
    //            shape_type: Plane,
    //            ..default()
    //        },
    //        Transform {
    //            translation: Vec3::new(0.0, 0.0, 0.0),
    //            rotation: Quat::from_axis_angle(Vec3::Y, -1.57),
    //            ..default()
    //        },
    //        GlobalTransform::IDENTITY,
    //    ));
    commands
        .spawn((
            Transform {
                //translation: Vec3::new(-2.0, -2.0, 0.0),
                //rotation: Quat::from_euler(EulerRot::XYZ, 0.5, 0.5, 0.0),
                scale: Vec3::new(1.0, 1.0, 1.0),
                rotation: Quat::from_euler(EulerRot::XYZ, 0.5, 0.0, 0.5),
                ..default()
            },
            GlobalTransform::IDENTITY,
        ))
        .with_children(|builder| {
            builder.spawn((
                Shape {
                    shape_type: Cube {
                        size: Vec3::new(1.0, 2.0, 1.0),
                    },
                    ..default()
                },
                Transform {
                    translation: Vec3::new(-2.0, -2.0, 0.0),
                    ..default()
                },
                GlobalTransform::IDENTITY,
                Bouncing,
            ));
        });
    commands
        .spawn((
            Shape {
                shape_type: Sphere { radius: 1.0 },
                ..default()
            },
            Transform {
                translation: Vec3::new(1.0, 0.0, 0.0),
                scale: Vec3::new(1.0, 1.0, 1.0),
                rotation: Quat::from_rotation_z(0.5),
                ..default()
            },
            GlobalTransform::IDENTITY,
        ))
        .with_children(|builder| {
            builder.spawn((
                Shape {
                    shape_type: Sphere { radius: 1.5 },
                    ..default()
                },
                Transform::from_xyz(0.0, 1.0, 0.0),
                GlobalTransform::IDENTITY,
                Bouncing,
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
        transform.translation.z = time.elapsed_seconds().sin() + 1.0;
    }
}
