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
    plane_count: u32,
    planes: array<Transform, #{MAX_PLANES}>,
    sphere_count: u32,
    spheres: array<Transform, #{MAX_SPHERES}>,
    cube_count: u32,
    cubes: array<Transform, #{MAX_CUBES}>,
};

struct Stage {
    texel_size: vec2<f32>,
}

@group(0) @binding(0)
var<uniform> camera: Camera;
@group(1) @binding(0)
var<uniform> shapes: Shapes;

#ifdef FIRST_STAGE
    @group(2) @binding(0)
    var<uniform> stage: Stage;
#else
    @group(2) @binding(0)
    var<uniform> stage: Stage;
    @group(2) @binding(1)
    var input_texture: texture_2d<f32>;
    @group(2) @binding(2)
    var input_sampler: sampler;
#endif

@fragment
fn main(@location(0) uv: vec2<f32>) ->
#ifdef LAST_STAGE
    @location(0) vec4<f32>
#else
    @location(0) f32
#endif
{
    let screen_uv = vec2(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0);
    let pos = camera.position;
    let dir = get_direction(screen_uv);
    let rad = sqrt(max(
        max(
            length_squared(dir - get_direction(screen_uv + vec2(-stage.texel_size.x, -stage.texel_size.y))),
            length_squared(dir - get_direction(screen_uv + vec2(stage.texel_size.x, -stage.texel_size.y)))
        ),
        max(
            length_squared(dir - get_direction(screen_uv + vec2(-stage.texel_size.x, stage.texel_size.y))),
            length_squared(dir - get_direction(screen_uv + vec2(stage.texel_size.x, stage.texel_size.y)))
        )
    ));

    #ifdef FIRST_STAGE
        var distance = 0.0;
    #else
        var distance = textureSample(input_texture, input_sampler, uv).r;
    #endif

    #ifndef LAST_STAGE
        for (var i = 0u; i < 8u; i = i + 1u) {
            let step = sdf(pos + dir * distance);
            distance = clamp((distance + step) / (1.0 + rad), distance, 1024.0);
        }
        return distance;
    #else
        var collided = false;
        while (distance < 1024.0) {
            let step = sdf(pos + dir * distance);
            if step >= distance * rad {
                distance = distance + step;
            } else {
                collided = true;
                break;
            }
        }

        if collided {
            let normal = normal(pos + dir * distance);
            let ambient_occlusion = ambient_occlusion(pos + dir * distance, normal);
            return vec4(vec3(0.6 + ambient_occlusion * 0.4), 1.0);
        } else {
            return vec4(vec3(0.0), 1.0);
        }
    #endif
}

// from: https://www.alanzucconi.com/2016/07/01/ambient-occlusion/
fn ambient_occlusion(pos: vec3<f32>, normal: vec3<f32>) -> f32 {
    // With the burnt in step size: 0.1
    // Unwrapped loop for 4 iterations
    // Precomputed inverse max_sum for 4 iterations:
    // 1 / ((1 / 2^0 + 2 / 2^1 + 3 / 2^2 + 4 / 2^3) * 0.1) = 3.07692307692

    var sum = max(0.0, sdf(pos + normal * 0.1));
    sum += max(0.0, sdf(pos + normal * 0.2)) * 0.5;
    sum += max(0.0, sdf(pos + normal * 0.3)) * 0.25;
    sum += max(0.0, sdf(pos + normal * 0.4)) * 0.125;
    return sum * 3.07692307692;
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

fn sdf(pos: vec3<f32>) -> f32 {
    var dist =
        sdf_plane(pos_transform(pos, shapes.planes[0].inv_transform)) *
        shapes.planes[0].min_scale;

    for (var i: u32 = 1u; i < shapes.plane_count; i = i + 1u) {
        var plane_dist =
            sdf_plane(pos_transform(pos, shapes.planes[i].inv_transform)) *
            shapes.planes[i].min_scale;

        dist = min(dist, plane_dist);
    }

    for (var i: u32 = 0u; i < shapes.sphere_count; i = i + 1u) {
        var sphere_dist =
            sdf_sphere(1.0, pos_transform(pos, shapes.spheres[i].inv_transform)) *
            shapes.spheres[i].min_scale;

        dist = min(dist, sphere_dist);
    }

    for (var i: u32 = 0u; i < shapes.cube_count; i = i + 1u) {
        var cube_dist =
            sdf_cube(vec3(1.0), pos_transform(pos, shapes.cubes[0].inv_transform)) *
            shapes.cubes[0].min_scale;

        dist = min(dist, cube_dist);
    }

    return dist;
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



fn get_direction(uv: vec2<f32>) -> vec3<f32> {
    return normalize(
        camera.right * uv.x +
        camera.up * uv.y +
        camera.forward
    );
}

fn length_squared(v: vec3<f32>) -> f32 {
    return dot(v, v);
}

fn pos_transform(pnt: vec3<f32>, transform: mat4x4<f32>) -> vec3<f32> {
    return (transform * vec4(pnt, 1.0)).xyz;
}

fn dir_transform(pnt: vec3<f32>, transform: mat4x4<f32>) -> vec3<f32> {
    return (transform * vec4(pnt, 0.0)).xyz;
}
