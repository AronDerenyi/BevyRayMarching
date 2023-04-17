use bevy::prelude::{Camera, Entity, Query, With};
use bevy_egui::{
    egui::{DragValue, Grid, Id, Window},
    EguiContexts,
};

use crate::ray_marching::RayMarching;

pub fn ui(
    mut egui_contexts: EguiContexts,
    mut views: Query<(Entity, &mut RayMarching), With<Camera>>,
) {
    for (entity, mut ray_marching) in views.iter_mut() {
        Window::new("View")
            .id(Id::new(entity))
            .collapsible(false)
            .show(egui_contexts.ctx_mut(), |ui| {
                Grid::new("resolution").num_columns(2).show(ui, |ui| {
                    ui.label("Resolution start:");
                    ui.add(
                        DragValue::new(&mut ray_marching.resolution_start)
                            .clamp_range(1..=32)
                            .speed(1),
                    );
                    ui.end_row();
                    ui.label("Resolution scaling:");
                    ui.add(
                        DragValue::new(&mut ray_marching.resolution_scaling)
                            .clamp_range(2..=8)
                            .speed(1),
                    );
                    ui.end_row();
                    ui.label("Resolution scale:");
                    ui.add(
                        DragValue::new(&mut ray_marching.resolution_scale)
                            .clamp_range(0.001..=1.0)
                            .speed(0.01),
                    );
                    ui.end_row();
                });
                ui.separator();
                Grid::new("iterations").num_columns(2).show(ui, |ui| {
                    ui.label("Iterations:");
                    ui.add(
                        DragValue::new(&mut ray_marching.iterations)
                            .clamp_range(1..=32)
                            .speed(1),
                    );
                    ui.end_row();
                });
                ui.separator();
                Grid::new("draw_mode").num_columns(2).show(ui, |ui| {
                    ui.label("Materials:");
                    ui.checkbox(&mut ray_marching.materials, "");
                    ui.end_row();
                    ui.label("Lighting:");
                    ui.checkbox(&mut ray_marching.lighting, "");
                    ui.end_row();
                    ui.label("Ambient occlusion:");
                    ui.checkbox(&mut ray_marching.ambient_occlusion, "");
                    ui.end_row();
                    ui.label("Debug iterations:");
                    ui.checkbox(&mut ray_marching.debug_iterations, "");
                    ui.end_row();
                });
            });
    }
}
