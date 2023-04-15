mod diagnostics;
mod shape;
mod shapes;

use bevy::prelude::{Entity, IntoSystemConfig, Plugin, Resource};

pub struct UIPlugin;

impl Plugin for UIPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.init_resource::<SelectedShape>().add_systems((
            diagnostics::ui,
            shapes::ui.after(diagnostics::ui),
            shape::ui.after(shapes::ui),
        ));
    }
}

#[derive(Resource, Default)]
pub struct SelectedShape(Option<Entity>);