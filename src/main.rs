pub mod ray_marching;

use std::f32::consts;

use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, Diagnostics};
use bevy::{input::mouse::MouseWheel, diagnostic::LogDiagnosticsPlugin};
use bevy::prelude::*;
use bevy::window::PresentMode;
use bevy_egui::{egui, EguiContext, EguiPlugin, EguiContexts};
use ray_marching::{
    shapes::Shape::{Cube, Plane, Sphere},
    RayMarchingPlugin,
};

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
        .init_resource::<FrameCount>()
        .add_system(performance)
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
                //                            viewport: Some(Viewport {
                //                                physical_position: UVec2::new(0, 0),
                //                                physical_size: UVec2::new(300, 300),
                //                                ..default()
                //                            }),
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
        Plane,
        Transform {
            translation: Vec3::new(0.0, 0.0, -3.0),
            ..default()
        },
        GlobalTransform::IDENTITY,
    ));
    commands
        .spawn((
            Transform {
                //translation: Vec3::new(-2.0, -2.0, 0.0),
                //rotation: Quat::from_euler(EulerRot::XYZ, 0.5, 0.5, 0.0),
                scale: Vec3::new(2.0, 1.0, 1.0),
                ..default()
            },
            GlobalTransform::IDENTITY,
        ))
        .with_children(|builder| {
            builder.spawn((
                Cube {
                    size: Vec3::new(1.0, 1.0, 1.0),
                },
                Transform {
                    translation: Vec3::new(-2.0, -2.0, 0.0),
                    rotation: Quat::from_euler(EulerRot::XYZ, 0.5, 0.0, 0.5),
                    ..default()
                },
                GlobalTransform::IDENTITY,
            ));
        });
    commands
        .spawn((
            Sphere { radius: 1.0 },
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
                Sphere { radius: 1.0 },
                Transform::from_xyz(0.0, 1.0, 0.0),
                GlobalTransform::IDENTITY,
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

#[derive(Resource, Default)]
struct FrameCount {
    frames: u32,
    elapsed: f32,
}

fn performance(time: Res<Time>, mut frame_count: ResMut<FrameCount>) {
    frame_count.frames += 1;
    frame_count.elapsed += time.delta_seconds();
    if frame_count.elapsed > 1.0 {
        frame_count.elapsed %= 1.0;
        println!("{}", frame_count.frames);
        frame_count.frames = 0;
    }
}
