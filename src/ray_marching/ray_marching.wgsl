#import bevy_core_pipeline::fullscreen_vertex_shader

struct TestUniform {
    value: f32
};

@group(0) @binding(0)
var<uniform> uni: TestUniform;

@fragment
fn main(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    return vec4<f32>(uv, uni.value, 1.0);
}
