use bevy::{asset::load_internal_asset, prelude::*};

use crate::vertex_shader;

const PRELUDE_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(11291576006157771079);

const SMUD_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(10055894596049459186);

const VIEW_BINDINGS_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(11792080578571156967);

/// The default fill used by `SmudShape`
pub const DEFAULT_FILL_HANDLE: Handle<Shader> = Handle::weak_from_u128(18184663565780163454);

/// Simple single-colored filled fill
pub const SIMPLE_FILL_HANDLE: Handle<Shader> = Handle::weak_from_u128(16286090377316294491);

pub const DEFAULT_SDF_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(87169507495872744019129766473752);

pub const fn get_vertex_handle<const F_PARAMS: usize, const U_PARAMS: usize>() -> Handle<Shader> {
    let id = 16846632126033267571u128; //this is the old shader uuid
    let new_id = id
        .wrapping_add((F_PARAMS.wrapping_mul(2875688479)) as u128)
        .wrapping_add((U_PARAMS.wrapping_mul(8693298443)) as u128);

    Handle::weak_from_u128(new_id)
}

pub struct ShaderLoadingPlugin<const F_PARAMS: usize, const U_PARAMS: usize>;

impl<const F_PARAMS: usize, const U_PARAMS: usize> Plugin
    for ShaderLoadingPlugin<F_PARAMS, U_PARAMS>
{
    fn build(&self, app: &mut App) {
        let vertex_shader = vertex_shader::create_vertex_shader::<F_PARAMS, U_PARAMS>();

        let mut shaders = app.world.resource_mut::<Assets<Shader>>();
        shaders.insert(get_vertex_handle::<F_PARAMS, U_PARAMS>(), vertex_shader);

        load_internal_asset!(
            app,
            PRELUDE_SHADER_HANDLE,
            "../assets/prelude.wgsl",
            Shader::from_wgsl
        );

        load_internal_asset!(
            app,
            SMUD_SHADER_HANDLE,
            "../assets/smud.wgsl",
            Shader::from_wgsl
        );

        load_internal_asset!(
            app,
            VIEW_BINDINGS_SHADER_HANDLE,
            "../assets/view_bindings.wgsl",
            Shader::from_wgsl
        );

        load_internal_asset!(
            app,
            DEFAULT_FILL_HANDLE,
            "../assets/fills/cubic_falloff.wgsl",
            Shader::from_wgsl
        );

        load_internal_asset!(
            app,
            SIMPLE_FILL_HANDLE,
            "../assets/fills/simple.wgsl",
            Shader::from_wgsl
        );

        load_internal_asset!(
            app,
            DEFAULT_SDF_HANDLE,
            "../assets/sdf/anywhere.wgsl",
            Shader::from_wgsl
        );
    }
}
