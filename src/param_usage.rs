use std::fmt::Debug;

use bevy::reflect::Reflect;

/// The type of a shader parameter
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Reflect)]
pub enum ShaderParamType {
    /// A 32 bit floating point number
    #[default]
    F32,
    /// A 32 bit unsigned integer
    U32,
}

/// Metadata about a parameter to a shader
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Reflect)]
pub struct ShaderParameter(u8, ShaderParamType);

impl ShaderParameter {
    /// Create an f32 parameter
    pub const fn f32(index: u8) -> Self {
        Self(index, ShaderParamType::F32)
    }

    /// Create a u32 parameter
    pub const fn u32(index: u8) -> Self {
        Self(index, ShaderParamType::U32)
    }
}

/// Describes which parameters need to be passed to the shader
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct ShaderParamUsage(pub &'static [ShaderParameter]);

impl ShaderParamUsage {
    /// Use no parameters
    pub const NO_PARAMS: Self = Self(&[]);

    /// Creates the params string to be passed to the sdf or fill function in the fragment shader
    pub(crate) fn in_params_str(self) -> String {
        let mut ret = "".to_string();

        for param in self.0 {
            let index = param.0;
            let s = match param.1 {
                ShaderParamType::F32 => format!(", in.param_f_{index}"),
                ShaderParamType::U32 => format!(", in.param_u_{index}"),
            };
            ret.push_str(s.as_str());
        }

        return ret;
    }

    pub(crate) fn func_def_arguments(self) -> String {
        let mut ret = "".to_string();

        for param in self.0 {
            let index = param.0;
            let s = match param.1 {
                ShaderParamType::F32 => format!(",param_f_{index}: f32"),
                ShaderParamType::U32 => format!(",param_u_{index}: u32"),
            };
            ret.push_str(s.as_str())
        }

        return ret;
    }
}
