use bevy::prelude::*;

use crate::{DEFAULT_FILL_HANDLE, param_usage::ShaderParamUsage};

#[derive(Component, Reflect, Debug, Clone)]
#[reflect(Component)]
/// Main component used for describing an sdf shape
pub struct SmudShape<const PARAMS: usize> {
    /// The color used by the fill shader
    pub color: Color,
    /// Shader containing a wgsl function for a signed distance field
    ///
    /// The shader needs to have the signature `fn sdf(p: vec2<f32>) -> f32`.
    pub sdf: Handle<Shader>,
    /// Shader containing a wgsl function for the fill of the shape
    ///
    /// The shader needs to have the signature `fn fill(distance: f32, color: vec4<f32>) -> vec4<f32>`.
    pub fill: Handle<Shader>, // todo: wrap in newtypes?
    /// The outer bounds for the shape, should be bigger than the sdf shape
    pub frame: Frame,
    /// Parameters to pass to shapes, for things such as width of a box
    // perhaps it would be a better idea to have this as a separate component?
    // keeping it here for now...
    pub params: [f32; PARAMS],

    /// Indicates which of the params will be passed into the sdf function
    pub sdf_param_usage: ShaderParamUsage,

    /// Indicates which of the params will be passed into the fill function
    pub fill_param_usage: ShaderParamUsage
}

impl<const PARAMS: usize> Default for SmudShape<PARAMS> {
    fn default() -> Self {
        Self {
            color: Color::PINK,
            sdf: default(),
            frame: default(),
            params: [0.0; PARAMS],
            fill: DEFAULT_FILL_HANDLE,
            sdf_param_usage: ShaderParamUsage::NO_PARAMS,
            fill_param_usage: ShaderParamUsage::NO_PARAMS,
        }
    }
}

/// Bounds for describing how far the fragment shader of a shape will reach, should be bigger than the shape unless you want to clip it
#[derive(Reflect, Debug, Clone, Copy)]
pub enum Frame {
    /// A quad with a given half-size (!)
    Quad(f32), // todo: it probably makes sense for this to be the full width instead...
}

impl Frame {
    const DEFAULT_QUAD: Self = Self::Quad(1.);
}

impl Default for Frame {
    fn default() -> Self {
        Self::DEFAULT_QUAD
    }
}
