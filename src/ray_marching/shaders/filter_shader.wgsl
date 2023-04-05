@group(0) @binding(0)
var input_texture: texture_2d<f32>;
@group(0) @binding(1)
var s: sampler;

@fragment
fn main(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    return textureSample(input_texture, s, uv).rgba;
}