#import bevy_core_pipeline::fullscreen_vertex_shader

struct View {
    position: vec3<f32>,
    right: vec3<f32>,
    up: vec3<f32>,
    forward: vec3<f32>,
};

struct Shapes {
    planes: array<Plane, #{MAX_PLANES}>,
    spheres: array<Sphere, #{MAX_SPHERES}>,
    cubes: array<Cube, #{MAX_CUBES}>,
};

struct Plane {
    inv_transform: mat4x4<f32>,
    scale: f32,
    material: Material,
};

struct Sphere {
    radius: f32,
    inv_transform: mat4x4<f32>,
    scale: f32,
    material: Material,
};

struct Cube {
    size: vec3<f32>,
    inv_transform: mat4x4<f32>,
    scale: f32,
    material: Material,
};

struct Material {
    color: vec3<f32>,
};

struct Stage {
    texel_size: vec2<f32>,
}

@group(0) @binding(0)
var<uniform> view: View;
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
    let pos = view.position;
    let dir = get_direction(screen_uv);

    #ifdef FIRST_STAGE
        var distance = 0.0;
    #else
        var distance = textureSample(input_texture, input_sampler, uv).r;
    #endif

    #ifndef LAST_STAGE
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

        for (var i = 0u; i < #{ITERATIONS}u; i = i + 1u) {
            let step = sdf(pos + dir * distance);
            distance = clamp((distance + step) / (1.0 + rad), distance, 1024.0);
        }
        return distance;
    #else
        let rad = sqrt(min(
            min(
                length_squared(dir - get_direction(screen_uv + vec2(-stage.texel_size.x, 0.0))),
                length_squared(dir - get_direction(screen_uv + vec2(stage.texel_size.x, 0.0)))
            ),
            min(
                length_squared(dir - get_direction(screen_uv + vec2(0.0, -stage.texel_size.y))),
                length_squared(dir - get_direction(screen_uv + vec2(0.0, stage.texel_size.y)))
            )
        ));

        #ifdef DEBUG_ITERATIONS
            var iterations = 0u;
        #endif

        var collided = false;
        while (distance < 1024.0) {
            #ifdef DEBUG_ITERATIONS
                iterations += 1u;
            #endif

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

            #ifdef MATERIALS
                let material = material(pos + dir * distance);
            #else
                let material = Material(vec3(1.0));
            #endif

            var color = material.color;

            #ifdef LIGHTING
                color *= (normal.z * 0.4 + 0.6);
            #endif

            #ifdef AMBIENT_OCCLUSION
                let ambient_occlusion = ambient_occlusion(pos + dir * distance, normal);
                color *= (0.6 + ambient_occlusion * 0.4);
            #endif

            #ifdef DEBUG_ITERATIONS
                let iterations = f32(iterations) / 32.0;
                color *= vec3(min(1.0, iterations), clamp(2.0 - iterations, 0.0, 1.0), 0.0);
            #endif

            return vec4(color, 1.0);
        } else {
            return vec4(vec3(0.0), 1.0);
        }
    #endif
}

// from: https://www.alanzucconi.com/2016/07/01/ambient-occlusion/
fn ambient_occlusion(pnt: vec3<f32>, normal: vec3<f32>) -> f32 {
    // With the burnt in step size: 0.1
    // Unwrapped loop for 4 iterations
    // Precomputed inverse max_sum for 4 iterations:
    // 1 / ((1 / 2^0 + 2 / 2^1 + 3 / 2^2 + 4 / 2^3) * 0.1) = 3.07692307692

    var sum = max(0.0, sdf(pnt + normal * 0.1));
    sum += max(0.0, sdf(pnt + normal * 0.2)) * 0.5;
    sum += max(0.0, sdf(pnt + normal * 0.3)) * 0.25;
    sum += max(0.0, sdf(pnt + normal * 0.4)) * 0.125;
    return sum * 3.07692307692;
}

fn normal(pnt: vec3<f32>) -> vec3<f32> {
    var epsilon = 0.001;
    return normalize(
        vec3(1.0, -1.0, -1.0) * sdf(pnt + vec3(1.0, -1.0, -1.0) * epsilon) +
        vec3(-1.0, 1.0, -1.0) * sdf(pnt + vec3(-1.0, 1.0, -1.0) * epsilon) +
        vec3(-1.0, -1.0, 1.0) * sdf(pnt + vec3(-1.0, -1.0, 1.0) * epsilon) +
        vec3(1.0, 1.0, 1.0) * sdf(pnt + vec3(1.0, 1.0, 1.0) * epsilon)
    );
}

fn sdf_plane(index: u32, pnt: vec3<f32>) -> f32 {
    let plane = &shapes.planes[index];
    return pos_transform(pnt, (*plane).inv_transform).z * (*plane).scale;
}

fn sdf_sphere(index: u32, pnt: vec3<f32>) -> f32 {
    let sphere = &shapes.spheres[index];
    return (length(pos_transform(pnt, (*sphere).inv_transform)) - (*sphere).radius) * (*sphere).scale;
}

fn sdf_cube(index: u32, pnt: vec3<f32>) -> f32 {
    let cube = &shapes.cubes[index];
    var q = abs(pos_transform(pnt, (*cube).inv_transform)) - (*cube).size;
    return (length(max(q, vec3(0.0))) + min(max(q.x, max(q.y, q.z)), 0.0)) * (*cube).scale;
}



fn min_select(left: ptr<function, f32>, right: f32) -> bool {
    *left = min(*left, right);
    return *left == right;
}

fn max_select(left: ptr<function, f32>, right: f32) -> bool {
    *left = max(*left, right);
    return *left == right;
}

fn get_direction(uv: vec2<f32>) -> vec3<f32> {
    return normalize(
        view.right * uv.x +
        view.up * uv.y +
        view.forward
    );
}

fn pos_transform(pnt: vec3<f32>, transform: mat4x4<f32>) -> vec3<f32> {
    return (transform * vec4(pnt, 1.0)).xyz;
}

fn dir_transform(pnt: vec3<f32>, transform: mat4x4<f32>) -> vec3<f32> {
    return (transform * vec4(pnt, 0.0)).xyz;
}

fn length_squared(v: vec3<f32>) -> f32 {
    return dot(v, v);
}
