use bevy::{
    asset::load_internal_asset,
    prelude::{App, HandleUntyped, Shader},
    reflect::TypeUuid,
};

pub const RAYMARCHING_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 698782022341067255);

pub const FILTER_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 698782022341067256);

pub const STENCIL_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 698782022341067257);

pub fn load_shaders(app: &mut App) {
    load_internal_asset!(
        app,
        RAYMARCHING_SHADER_HANDLE,
        "ray_marching.wgsl",
        Shader::from_wgsl
    );
    load_internal_asset!(
        app,
        FILTER_SHADER_HANDLE,
        "filter_shader.wgsl",
        Shader::from_wgsl
    );
    load_internal_asset!(
        app,
        STENCIL_SHADER_HANDLE,
        "stencil_shader.wgsl",
        Shader::from_wgsl
    );
}
