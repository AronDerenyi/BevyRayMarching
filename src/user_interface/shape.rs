use super::SelectedShape;
use crate::{
    ray_marching::{
        Material,
        Operation::{Intersection, Union, SmoothUnion},
        Primitive::{Cube, Image, Plane, Sphere},
        Shape, ShapeImage,
        ShapeType::{self, Compound, Primitive},
    },
    Images,
};
use bevy::prelude::{
    BuildChildren, Commands, DespawnRecursiveExt, Entity, EulerRot, Handle, Mut, Name, Parent,
    Quat, Query, Res, Transform, Vec3,
};
use bevy_egui::{
    egui::{Align, ComboBox, DragValue, Grid, Layout, Ui, Window},
    EguiContexts,
};

pub fn ui(
    mut commands: Commands,
    mut egui_contexts: EguiContexts,
    selected_shape: Res<SelectedShape>,
    images: Res<Images>,
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
            shape_ui(ui, &images, shape);
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

fn shape_ui(ui: &mut Ui, images: &Images, mut shape: Mut<Shape>) {
    Grid::new("shape").num_columns(2).show(ui, |ui| {
        ui.label("Type:");
        shape_type_ui(ui, images, &mut shape.shape_type);
        ui.end_row();

        if let Primitive(ref mut primitive, ref mut material) = &mut shape.shape_type {
            match primitive {
                Sphere { ref mut radius } => {
                    ui.label("Radius:");
                    scalar_ui(ui, radius);
                    ui.end_row();
                }
                Cube { ref mut size } => {
                    ui.label("Size:");
                    vec_ui(ui, size);
                    ui.end_row();
                }
                _ => {}
            }
            ui.label("Color:");
            color_ui(ui, &mut material.color);
            ui.end_row();
        }

        ui.label("Negative:");
        ui.checkbox(&mut shape.negative, "");
        ui.end_row();
    });
}

fn shape_type_ui(ui: &mut Ui, images: &Images, shape_type: &mut ShapeType) {
    let (name, radius, size, handle, material) = match shape_type {
        Primitive(Plane, material) => {
            ("Plane", 1.0, Vec3::ONE, Handle::default(), material.clone())
        }
        Primitive(Sphere { radius }, material) => (
            "Sphere",
            *radius,
            Vec3::splat(*radius),
            Handle::default(),
            material.clone(),
        ),
        Primitive(Cube { size }, material) => (
            "Cube",
            (size.x + size.y + size.z) / 3.0,
            *size,
            Handle::default(),
            material.clone(),
        ),
        Primitive(Image(image_handle), material) => (
            images
                .iter()
                .find(|(_, handle)| handle == image_handle)
                .map_or("Image", |(name, _)| name),
            1.0,
            Vec3::ONE,
            image_handle.clone(),
            material.clone(),
        ),
        Compound(Union) => (
            "Union",
            1.0,
            Vec3::ONE,
            Handle::default(),
            Material::default(),
        ),
        Compound(Intersection) => (
            "Intersection",
            1.0,
            Vec3::ONE,
            Handle::default(),
            Material::default(),
        ),
        Compound(SmoothUnion) => (
            "SmoothUnion",
            1.0,
            Vec3::ONE,
            Handle::default(),
            Material::default(),
        ),
    };

    ComboBox::new("shape_type", "")
        .selected_text(name)
        .show_ui(ui, |ui| {
            ui.selectable_value(shape_type, Primitive(Plane, material.clone()), "Plane");
            ui.selectable_value(
                shape_type,
                Primitive(Sphere { radius }, material.clone()),
                "Sphere",
            );
            ui.selectable_value(
                shape_type,
                Primitive(Cube { size }, material.clone()),
                "Cube",
            );
            for (name, handle) in images.iter() {
                ui.selectable_value(
                    shape_type,
                    Primitive(Image(handle.clone()), material.clone()),
                    name,
                );
            }
            ui.selectable_value(shape_type, Compound(Union), "Union");
            ui.selectable_value(shape_type, Compound(Intersection), "Intersection");
            ui.selectable_value(shape_type, Compound(SmoothUnion), "SmoothUnion");
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

fn color_ui(ui: &mut Ui, vec: &mut Vec3) {
    ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
        ui.label("R:");
        scalar_ui(ui, &mut vec.x);
        ui.label("G:");
        scalar_ui(ui, &mut vec.y);
        ui.label("B:");
        scalar_ui(ui, &mut vec.z);
    });
}
