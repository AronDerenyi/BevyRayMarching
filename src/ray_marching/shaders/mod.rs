use bevy::{
    asset::load_internal_asset,
    prelude::{App, HandleUntyped, Shader},
    reflect::TypeUuid,
};

pub const TRACING_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 698782022341067255);

pub const UPSAMPLING_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 698782022341067256);

pub fn load_shaders(app: &mut App) {
    load_internal_asset!(
        app,
        TRACING_SHADER_HANDLE,
        "tracing.wgsl",
        Shader::from_wgsl
    );
    load_internal_asset!(
        app,
        UPSAMPLING_SHADER_HANDLE,
        "upsampling.wgsl",
        Shader::from_wgsl
    );
}
