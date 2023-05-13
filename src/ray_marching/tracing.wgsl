#import bevy_core_pipeline::fullscreen_vertex_shader

struct View {
    position: vec3<f32>,
    right: vec3<f32>,
    up: vec3<f32>,
    forward: vec3<f32>,
};

struct Environment {
    sky: vec3<f32>,
    sun_direction: vec3<f32>,
    sun_light: vec3<f32>,
};

struct Shapes {
    planes: array<Plane, #{MAX_PLANES}>,
    spheres: array<Sphere, #{MAX_SPHERES}>,
    cubes: array<Cube, #{MAX_CUBES}>,
    images: array<Image, #{MAX_IMAGES}>,
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
    bounds: vec3<f32>,
    inv_transform: mat4x4<f32>,
    scale: f32,
    material: Material,
};

struct Image {
    bounds: vec3<f32>,
    texture_bounds: vec3<f32>,
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
@group(1) @binding(1)
var shape_sampler: sampler;
@group(1) @binding(2)
var shape_texture: texture_3d<f32>;

#ifdef FIRST_STAGE
    @group(2) @binding(0)
    var<uniform> stage: Stage;
#else
    #ifndef LAST_STAGE
        @group(2) @binding(0)
        var<uniform> stage: Stage;
        @group(2) @binding(1)
        var input_texture: texture_2d<f32>;
        @group(2) @binding(2)
        var input_sampler: sampler;
    #else
        @group(2) @binding(0)
        var<uniform> environment: Environment;
        @group(3) @binding(0)
        var<uniform> stage: Stage;
        @group(3) @binding(1)
        var input_texture: texture_2d<f32>;
        @group(3) @binding(2)
        var input_sampler: sampler;
    #endif
#endif

struct SDFMaterialResult {
    distance: f32,
    material: Material,
}

#ifdef DEBUG_ITERATIONS
    var<private> iterations: u32 = 0u;
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
        var progress = 0.0;
    #else
        var progress = textureSample(input_texture, input_sampler, uv).r;
    #endif

    #ifndef LAST_STAGE
        let texel_radius = sqrt(max(
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
            let distance = sdf(pos + dir * progress);
            progress = clamp((progress + distance) / (1.0 + texel_radius), progress, #{FAR}f);
        }
        return progress;
    #else
        let texel_radius = sqrt(min(
            min(
                length_squared(dir - get_direction(screen_uv + vec2(-stage.texel_size.x, 0.0))),
                length_squared(dir - get_direction(screen_uv + vec2(stage.texel_size.x, 0.0)))
            ),
            min(
                length_squared(dir - get_direction(screen_uv + vec2(0.0, -stage.texel_size.y))),
                length_squared(dir - get_direction(screen_uv + vec2(0.0, stage.texel_size.y)))
            )
        ));

        var collided = false;
        var distance: f32;
        while (progress < #{FAR}f) {
            #ifdef DEBUG_ITERATIONS
                iterations += 1u;
            #endif

            distance = sdf(pos + dir * progress);
            if distance >= progress * texel_radius {
                progress = progress + distance;
            } else {
                collided = true;
                break;
            }
        }

        #ifdef DEBUG_SDF
            let sdf_plane_progress = pos.z / -dir.z;
            let sdf_plane_collided = sdf_plane_progress >= 0.0 && sdf_plane_progress < progress;
        #endif

        var color: vec3<f32>;
        if collided {
            var pnt = pos + dir * progress;
            let normal = normal(pnt);
            pnt += normal * -distance;

            #ifdef MATERIALS
                let material = sdf_material(pnt).material;
                let diffuse_color = material.color;
            #else
                let diffuse_color = vec3(1.0);
            #endif

            #ifdef LIGHTING
                var indirect_light = diffuse_color * environment.sky * 0.5;
                #ifdef AMBIENT_OCCLUSION
                    indirect_light *= ambient_occlusion(pnt, normal);
                #endif

                let NoL = saturate(dot(normal, environment.sun_direction));
                let diffuse_light = environment.sun_light * NoL;
                var direct_light = diffuse_color * diffuse_light;
                #ifdef SHADOW
                    if NoL > 0.0 {
                        direct_light *= shadow(
                            pnt, environment.sun_direction, NoL,
                            max(0.02, progress * texel_radius), 0.02
                        );
                    }
                #endif

                color = direct_light + indirect_light;
            #else
                color = diffuse_color;
            #endif
        } else {
            #ifdef LIGHTING
                color = environment.sky;
            #else
                color = vec3(1.0);
            #endif
        }

        #ifdef DEBUG_ITERATIONS
            let max_iterations = 256u;
            let mapped_iterations = log2(f32(iterations)) / log2(f32(max_iterations)) * 2.0;
            color = vec3(
                saturate(mapped_iterations),
                saturate(2.0 - mapped_iterations),
                select(0.0, 1.0, iterations > max_iterations)
            );
        #endif

        #ifdef DEBUG_SDF
            if sdf_plane_collided {
                let pnt = pos + dir * sdf_plane_progress;
                #ifdef MATERIALS
                    let result = sdf_material(pnt);
                    let diffuse_color = result.material.color;
                    color = diffuse_color * ((result.distance * 2.0) % 1.0);
                #else
                    let sdf = sdf(pnt);
                    color = vec3((sdf * 2.0) % 1.0);
                #endif
            }
        #endif

        return vec4(color, 1.0);
    #endif
}

fn normal(pnt: vec3<f32>) -> vec3<f32> {
    var epsilon = 0.01;
    return normalize(
        vec3(1.0, -1.0, -1.0) * sdf(pnt + vec3(1.0, -1.0, -1.0) * epsilon) +
        vec3(-1.0, 1.0, -1.0) * sdf(pnt + vec3(-1.0, 1.0, -1.0) * epsilon) +
        vec3(-1.0, -1.0, 1.0) * sdf(pnt + vec3(-1.0, -1.0, 1.0) * epsilon) +
        vec3(1.0, 1.0, 1.0) * sdf(pnt + vec3(1.0, 1.0, 1.0) * epsilon)
    );
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
    return 0.6 + sum * 3.07692307692 * 0.4;
}

fn shadow(pnt: vec3<f32>, incident_light: vec3<f32>, NoL: f32, rad: f32, penumbra: f32) -> f32 {
    var progress = (rad + penumbra) / NoL;
    var min_distance = #{FAR}f;
    while (min_distance >= rad && progress < #{FAR}f) {
        #ifdef DEBUG_ITERATIONS
            iterations += 1u;
        #endif
        let distance = sdf(pnt + incident_light * progress);
        min_distance = min(min_distance, distance);
        progress += distance;
    }
    return saturate((min_distance - rad) / penumbra);
}



fn sdf(pnt: vec3<f32>) -> f32 {
    return sdf_generated(pnt);
}

fn sdf_material(pnt: vec3<f32>) -> SDFMaterialResult {
    return sdf_material_generated(pnt);
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
    let q = abs(pos_transform(pnt, (*cube).inv_transform)) - (*cube).bounds;
    return (length(max(q, vec3(0.0))) + min(max(q.x, max(q.y, q.z)), 0.0)) * (*cube).scale;
}

fn sdf_image(index: u32, pnt: vec3<f32>) -> f32 {
    let image = &shapes.images[index];
    let transformed_pnt = pos_transform(pnt, (*image).inv_transform);
    let q = abs(transformed_pnt) - (*image).bounds;
    let cube_distance = length(max(q, vec3(0.0))) + min(max(q.x, max(q.y, q.z)), 0.0);
    let image_distance = textureSample(
        shape_texture, shape_sampler,
        (transformed_pnt / (*image).texture_bounds + vec3(1.0)) * 0.5
    ).r;
    return select(
        image_distance,
        length(vec2(cube_distance, image_distance)),
        cube_distance > 0.0
    ) * (*image).scale;
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
