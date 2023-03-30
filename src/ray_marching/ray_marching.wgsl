#import bevy_core_pipeline::fullscreen_vertex_shader

struct Camera {
    position: vec3<f32>,
    right: vec3<f32>,
    up: vec3<f32>,
    forward: vec3<f32>,
};

struct Transform {
    inv_transform: mat4x4<f32>,
    min_scale: f32,
};

struct Shapes {
    plane: Transform,
    spheres: array<Transform, 2>,
    cubes: array<Transform, 1>,
};

@group(0) @binding(0)
var<uniform> camera: Camera;
@group(1) @binding(0)
var<uniform> shapes: Shapes;

fn pos_transform(pnt: vec3<f32>, transform: mat4x4<f32>) -> vec3<f32> {
    return (transform * vec4(pnt, 1.0)).xyz;
}

fn dir_transform(pnt: vec3<f32>, transform: mat4x4<f32>) -> vec3<f32> {
    return (transform * vec4(pnt, 0.0)).xyz;
}

fn sdf_plane(pnt: vec3<f32>) -> f32 {
    return pnt.z;
}

fn sdf_sphere(radius: f32, pnt: vec3<f32>) -> f32 {
    return (length(pnt) - radius);
}

fn sdf_cube(size: vec3<f32>, pnt: vec3<f32>) -> f32 {
    var q = abs(pnt) - size;
    return length(max(q, vec3(0.0))) + min(max(q.x, max(q.y, q.z)), 0.0);
}

fn sdf(pos: vec3<f32>) -> f32 {
    var dist =
        sdf_plane(pos_transform(pos, shapes.plane.inv_transform)) *
        shapes.plane.min_scale;

    for (var i: u32 = 0u; i < 2u; i = i + 1u) {
        var sphere_dist =
            sdf_sphere(1.0, pos_transform(pos, shapes.spheres[i].inv_transform)) *
            shapes.spheres[i].min_scale;

        dist = min(dist, sphere_dist);
    }

    for (var i: u32 = 0u; i < 1u; i = i + 1u) {
        var cube_dist =
            sdf_cube(vec3(1.0), pos_transform(pos, shapes.cubes[i].inv_transform)) *
            shapes.cubes[i].min_scale;

        dist = min(dist, cube_dist);
    }

    return dist;
}

fn normal(pos: vec3<f32>) -> vec3<f32> {
    var epsilon = 0.001;
    return normalize(
        vec3(1.0, -1.0, -1.0) * sdf(pos + vec3(1.0, -1.0, -1.0) * epsilon) +
        vec3(-1.0, 1.0, -1.0) * sdf(pos + vec3(-1.0, 1.0, -1.0) * epsilon) +
        vec3(-1.0, -1.0, 1.0) * sdf(pos + vec3(-1.0, -1.0, 1.0) * epsilon) +
        vec3(1.0, 1.0, 1.0) * sdf(pos + vec3(1.0, 1.0, 1.0) * epsilon)
    );
}

@fragment
fn main(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    var dir = normalize(
        camera.right * (uv.x * 2.0 - 1.0) +
        camera.up * (1.0 - uv.y * 2.0) +
        camera.forward
    );
    var pos = camera.position;

    var progress = 0.0;
    var dist = 0.0;
    for (var i: u32 = 0u; i < 64u; i = i + 1u) {
        dist = max(0.0, sdf(pos + dir * progress));
        progress = progress + dist;
    }

    var collided = dist < 0.05;
    if !collided {
        var transformed_pos = pos_transform(pos, shapes.plane.inv_transform);
        var transformed_dir = dir_transform(dir, shapes.plane.inv_transform);
        if transformed_dir.z < 0.0 {
            progress = transformed_pos.z / -transformed_dir.z;
            collided = true;
        }
    }

    if collided {
        return vec4(normal(pos + dir * progress) / 2.0 + vec3(.5, .5, .5), 1.0);
/*
        var sun = normalize(vec3(0.3, 0.5, 1.0));
        var norm = normal(pos + dir * progress);
        var shadow_pos = pos + dir * progress + norm * 0.01;
        var shadow_dir = sun;
        var shadow_progress = 0.0;
        var shadow_dist = 0.0;
        for (var i: u32 = 0u; i < 32u; i = i + 1u) {
            shadow_dist = sdf(shadow_pos + shadow_dir * shadow_progress);
            shadow_progress = shadow_progress + shadow_dist;
        }

        if shadow_dist < 0.1 {
            return vec4(vec3(dot(norm, sun) * 0.1), 1.0);
        } else {
            return vec4(vec3(dot(norm, sun)), 1.0);
        }*/

        //return vec4((pos + dir * progress) % 1.0, 1.0);
    } else {
        return vec4(0.0, 0.0, 0.0, 1.0);
    }
}



/* SDF with precalculated transformed rays
struct Ray {
    pos: vec3<f32>,
    dir: vec3<f32>,
}

struct ModelSpaceRays {
    plane: Ray,
    spheres: array<Ray, #{SPHERES}u>,
}

var<private> model_space_rays: ModelSpaceRays;

fn sdf(dist: f32) -> f32 {
    var d = sdf_plane(model_space_rays.plane.pos + model_space_rays.plane.dir * dist) * shapes.plane.min_scale;
    for (var i: u32 = 0u; i < 2u; i = i + 1u) {
        d = min(d, sdf_sphere(model_space_rays.spheres[i].pos + model_space_rays.spheres[i].dir * dist)) * shapes.spheres[i].min_scale;
    }
    return d;
}

// In main

model_space_rays.plane = Ray (
    (shapes.plane.inv_transform * vec4<f32>(pos, 1.0)).xyz,
    (shapes.plane.inv_transform * vec4<f32>(dir, 0.0)).xyz,
);

for (var i: u32 = 0u; i < #{SPHERES}u; i = i + 1u) {
    model_space_rays.spheres[i] = Ray (
        (shapes.spheres[i].inv_transform * vec4<f32>(pos, 1.0)).xyz,
        (shapes.spheres[i].inv_transform * vec4<f32>(dir, 0.0)).xyz,
    );
}
*/
