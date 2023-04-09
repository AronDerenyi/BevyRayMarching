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
    let pixel = uv / stage.texel_size;
    let m = ((floor(pixel.x) + floor(pixel.y)) % 2.0);

    #ifndef FIRST_STAGE
        let input = textureSample(input_texture, input_sampler, uv).r;
    #endif

    if m > 0.0 {
        #ifdef FIRST_STAGE
            return 1.0;
        #else
            #ifdef LAST_STAGE
                return vec4(0.0, input * 0.5 + 0.5, 0.0, 1.0);
            #else
                return input * 0.5 + 0.5;
            #endif
        #endif
    } else {
        #ifdef FIRST_STAGE
            return 0.0;
        #else
            #ifdef LAST_STAGE
                return vec4(0.0, input * 0.5, 0.0, 1.0);
            #else
                return input * 0.5;
            #endif
        #endif
    }
}