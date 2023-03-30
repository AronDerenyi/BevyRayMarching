@fragment
fn write(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    if length(uv - vec2(0.5)) > 0.2 {
        discard;
    }
    return vec4(uv, 0.0, 1.0);
}

@fragment
fn test(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    return vec4(uv, 1.0, 1.0);
}