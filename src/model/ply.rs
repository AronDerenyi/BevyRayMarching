use bevy::{prelude::Vec3, asset::Error};

use super::Model;

pub fn load(string: String) -> Result<Model, Error> {
    let mut lines = string.lines();
    if lines.next() != Some("ply") {
        return Err(Error::msg("not a ply file"));
    }
    if lines.next() != Some("format ascii 1.0") {
        return Err(Error::msg("not an ascii encoded ply file"));
    }

    let mut header = true;
    let mut property_index = 0;
    let mut x_index = -1;
    let mut y_index = -1;
    let mut z_index = -1;
    let mut vertex_count = -1;
    let mut face_count = -1;

    let mut vertices = Vec::<Vec3>::new();
    let mut model = Model::new();

    for line in lines {
        let split = line.split(" ").collect::<Vec<_>>();
        if header {
            if split[0] == "comment" {
                continue;
            }
            if split[0] == "end_header" {
                if x_index == -1 {
                    return Err(Error::msg("missing x property"));
                }
                if y_index == -1 {
                    return Err(Error::msg("missing y property"));
                }
                if z_index == -1 {
                    return Err(Error::msg("missing z property"));
                }
                if vertex_count == -1 {
                    return Err(Error::msg("missing vertex count"));
                }
                if face_count == -1 {
                    return Err(Error::msg("missing face count"));
                }

                header = false;
                continue;
            }

            match (split[0], split[1]) {
                ("element", "vertex") => vertex_count = split[2].parse()?,
                ("element", "face") => face_count = split[2].parse()?,
                ("property", "float") => {
                    match split[2] {
                        "x" => x_index = property_index,
                        "y" => y_index = property_index,
                        "z" => z_index = property_index,
                        _ => {}
                    }
                    property_index += 1;
                }
                ("property", "list") => {
                    if split[2] != "uchar" || split[3] != "uint" || split[4] != "vertex_indices" {
                        return Err(Error::msg(format!("invalid property list: {line}")));
                    }
                }
                ("property", t) => return Err(Error::msg(format!("invalid property type: {t}"))),
                _ => return Err(Error::msg(format!("invalid header line: {line}"))),
            }
        } else {
            if vertices.len() < vertex_count as usize {
                vertices.push(Vec3::new(
                    split[x_index as usize].parse()?,
                    split[y_index as usize].parse()?,
                    split[z_index as usize].parse()?,
                ));
            } else {
                if split[0] != "3" {
                    return Err(Error::msg(format!("non triangular face")));
                }
                model.push_triangle(
                    vertices[split[1].parse::<usize>()?],
                    vertices[split[2].parse::<usize>()?],
                    vertices[split[3].parse::<usize>()?],
                );
            }
        }
    }

    Ok(model)
}