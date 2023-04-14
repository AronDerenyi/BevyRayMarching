use super::SelectedShape;
use crate::ray_marching::{Shape, ShapeType};
use bevy::prelude::{
    BuildChildren, Commands, DespawnRecursiveExt, Entity, EulerRot, Mut, Name, Parent, Quat, Query,
    Res, Transform, Vec3,
};
use bevy_egui::{
    egui::{Align, ComboBox, DragValue, Grid, Layout, Ui, Window},
    EguiContexts,
};

pub fn ui(
    mut commands: Commands,
    mut egui_contexts: EguiContexts,
    selected_shape: Res<SelectedShape>,
    mut shapes: Query<(
        Entity,
        &mut Name,
        Option<&Parent>,
        &mut Transform,
        &mut Shape,
    )>,
) {
    let Some(selected_entity) = selected_shape.0 else {
        return
    };

    let parents = shapes
        .iter()
        .filter_map(|(entity, name, ..)| {
            if entity == selected_entity {
                None
            } else {
                Some((entity, name.to_string()))
            }
        })
        .collect::<Vec<(Entity, String)>>();

    let Ok((entity, name, parent, transform, shape)) = shapes.get_mut(selected_entity) else {
        return
    };

    Window::new("Shape")
        .collapsible(false)
        .show(egui_contexts.ctx_mut(), |ui| {
            name_parent_ui(ui, &mut commands, &parents, entity, name, parent);
            ui.separator();
            transform_ui(ui, transform);
            ui.separator();
            shape_ui(ui, shape);
            ui.separator();
            if ui.button("Delete").clicked() {
                commands.entity(entity).despawn_recursive();
            }
        });
}

fn name_parent_ui(
    ui: &mut Ui,
    commands: &mut Commands,
    parents: &Vec<(Entity, String)>,
    entity: Entity,
    mut name: Mut<Name>,
    parent: Option<&Parent>,
) {
    Grid::new("name_parent").num_columns(2).show(ui, |ui| {
        ui.label("Name:");
        let mut name_string = name.to_string();
        if ui.text_edit_singleline(&mut name_string).changed() {
            name.set(name_string);
        }
        ui.end_row();

        ui.label("Parent:");
        let mut parent = parent.map(|parent| parent.get());
        let mut changed = false;
        ComboBox::new("parent", "")
            .selected_text(
                    parent
                    .map(|parent| {
                        parents
                            .iter()
                            .find(|(entity, _)| *entity == parent)
                            .map(|(_, name)| name.as_str())
                    })
                    .flatten()
                    .unwrap_or("none"),
            )
            .show_ui(ui, |ui| {
                changed |= ui.selectable_value(&mut parent, None, "none").changed();
                for (entity, name) in parents {
                    changed |= ui
                        .selectable_value(&mut parent, Some(*entity), name)
                        .changed();
                }
            });
        if changed {
            match parent {
                Some(parent) => commands.entity(entity).set_parent(parent),
                None => commands.entity(entity).remove_parent(),
            };
        }
        ui.end_row();
    });
}

fn transform_ui(ui: &mut Ui, mut transform: Mut<Transform>) {
    Grid::new("transform").num_columns(2).show(ui, |ui| {
        ui.label("Position:");
        vec_ui(ui, &mut transform.translation);
        ui.end_row();
        ui.label("Rotation:");
        quat_ui(ui, &mut transform.rotation);
        ui.end_row();
        ui.label("Scale:");
        vec_ui(ui, &mut transform.scale);
        ui.end_row();
    });
}

fn shape_ui(ui: &mut Ui, mut shape: Mut<Shape>) {
    Grid::new("shape").num_columns(2).show(ui, |ui| {
        ui.label("Type:");
        shape_type_ui(ui, &mut shape.shape_type);
        ui.end_row();

        match shape.shape_type {
            ShapeType::Sphere { ref mut radius } => {
                ui.label("Radius:");
                scalar_ui(ui, radius);
                ui.end_row();
            }
            ShapeType::Cube { ref mut size } => {
                ui.label("Size:");
                vec_ui(ui, size);
                ui.end_row();
            }
            _ => {}
        }

        ui.label("Negative:");
        ui.checkbox(&mut shape.negative, "");
        ui.end_row();
    });
}

fn shape_type_ui(ui: &mut Ui, shape_type: &mut ShapeType) {
    let (name, radius, size) = match shape_type {
        ShapeType::Plane => ("Plane", 1.0, Vec3::ONE),
        ShapeType::Sphere { radius } => ("Sphere", *radius, Vec3::splat(*radius)),
        ShapeType::Cube { size } => ("Cube", (size.x + size.y + size.z) / 3.0, *size),
        ShapeType::Union => ("Union", 1.0, Vec3::ONE),
        ShapeType::Intersection => ("Intersection", 1.0, Vec3::ONE),
    };

    ComboBox::new("shape_type", "")
        .selected_text(name)
        .show_ui(ui, |ui| {
            ui.selectable_value(shape_type, ShapeType::Plane, "Plane");
            ui.selectable_value(shape_type, ShapeType::Sphere { radius }, "Sphere");
            ui.selectable_value(shape_type, ShapeType::Cube { size }, "Cube");
            ui.selectable_value(shape_type, ShapeType::Union, "Union");
            ui.selectable_value(shape_type, ShapeType::Intersection, "Intersection");
        });
}

fn scalar_ui(ui: &mut Ui, scalar: &mut f32) {
    ui.add(DragValue::new(scalar).speed(0.01));
}

fn vec_ui(ui: &mut Ui, vec: &mut Vec3) {
    ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
        ui.label("X:");
        scalar_ui(ui, &mut vec.x);
        ui.label("Y:");
        scalar_ui(ui, &mut vec.y);
        ui.label("Z:");
        scalar_ui(ui, &mut vec.z);
    });
}

fn quat_ui(ui: &mut Ui, quat: &mut Quat) {
    let mut vec = Vec3::from(quat.to_euler(EulerRot::XYZ));
    vec_ui(ui, &mut vec);
    *quat = Quat::from_euler(EulerRot::XYZ, vec.x, vec.y, vec.z);
}
