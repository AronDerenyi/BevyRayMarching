use super::SelectedShape;
use crate::ray_marching::{
    Operation::{Intersection, Union},
    Primitive::{Cube, Plane, Sphere},
    Shape,
    ShapeType::{self, Compound, Primitive},
};
use bevy::prelude::{
    Children, Commands, Entity, GlobalTransform, Name, Parent, Query, ResMut, Transform, With,
    Without,
};
use bevy_egui::{
    egui::{collapsing_header::CollapsingState, Id, Label, Sense, Ui, Window},
    EguiContexts,
};

pub fn ui(
    mut commands: Commands,
    mut egui_contexts: EguiContexts,
    mut selected_shape: ResMut<SelectedShape>,
    roots: Query<Entity, (With<Shape>, With<Transform>, Without<Parent>)>,
    shapes: Query<(Entity, &Name, &Shape, Option<&Children>)>,
) {
    Window::new("Shapes")
        .collapsible(false)
        .show(egui_contexts.ctx_mut(), |ui| {
            for entity in roots.iter() {
                shape_ui(ui, &mut selected_shape, entity, &shapes);
            }
            if ui.button("Add").clicked() {
                let entity = commands
                    .spawn((
                        Name::new("Shape"),
                        Shape::default(),
                        Transform::default(),
                        GlobalTransform::default(),
                    ))
                    .id();
                selected_shape.0 = Some(entity);
            }
        });
}

fn shape_ui(
    ui: &mut Ui,
    selected_shape: &mut ResMut<SelectedShape>,
    entity: Entity,
    shapes: &Query<(Entity, &Name, &Shape, Option<&Children>)>,
) {
    let Ok((entity, name, shape, children)) = shapes.get(entity) else {
        return;
    };

    if let Some(children) = children {
        CollapsingState::load_with_default_open(ui.ctx(), Id::new(entity), true)
            .show_header(ui, |ui| {
                shape_label_ui(ui, selected_shape, entity, name, shape);
            })
            .body(|ui| {
                for entity in children.into_iter() {
                    shape_ui(ui, selected_shape, *entity, shapes);
                }
            });
    } else {
        shape_label_ui(ui, selected_shape, entity, name, shape);
    }
}

fn shape_label_ui(
    ui: &mut Ui,
    selected_shape: &mut ResMut<SelectedShape>,
    entity: Entity,
    name: &Name,
    shape: &Shape,
) {
    ui.selectable_value(
        &mut selected_shape.0,
        Some(entity),
        match shape.shape_type {
            Primitive(Plane, ..) => format!("{name} (Plane)"),
            Primitive(Sphere { .. }, ..) => format!("{name} (Sphere)"),
            Primitive(Cube { .. }, ..) => format!("{name} (Cube)"),
            Compound(Union) => format!("{name} (Union)"),
            Compound(Intersection) => format!("{name} (Intersection)"),
        },
    );
}
