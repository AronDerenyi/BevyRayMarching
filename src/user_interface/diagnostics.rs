use bevy::{
    diagnostic::{Diagnostics, FrameTimeDiagnosticsPlugin},
    prelude::Res,
};
use bevy_egui::{egui::Window, EguiContexts};

pub fn ui(mut egui_contexts: EguiContexts, diagnostics: Res<Diagnostics>) {
    Window::new("Diagnostics").collapsible(false).show(egui_contexts.ctx_mut(), |ui| {
        if let Some(fps) = diagnostics.get(FrameTimeDiagnosticsPlugin::FPS) {
            ui.label(format!("{:.0} fps", fps.smoothed().unwrap_or(0.0)));
        }
        if let Some(frame_time) = diagnostics.get(FrameTimeDiagnosticsPlugin::FRAME_TIME) {
            ui.label(format!("{:.2} ms", frame_time.smoothed().unwrap_or(0.0)));
        }
    });
}
