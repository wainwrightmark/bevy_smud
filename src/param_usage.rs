use std::fmt::Debug;

use bevy::reflect::Reflect;

/// Describes which parameters need to be passed to the shader
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct ShaderParamUsage{
    /// An array of additional parameters to be passed to the shader in order
    pub parameters: &'static [ShaderParameter]
}

/// Metadata about a parameter to a shader
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Reflect)]
pub struct ShaderParameter{
    /// The index of the parameter to use
    pub index: u8,
    /// The type of the parameter
    pub param_type: ShaderParamType
}

/// The type of a shader parameter
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Reflect)]
pub enum ShaderParamType{
    /// A 32 bit floating point number
    #[default]
    F32,
    /// A 32 bit unsigned integer
    U32
}

impl ShaderParamUsage {
    /// Use no parameters
    pub const NO_PARAMS: Self = Self{parameters: &[]};


    /// Creates the params string to be passed to the sdf or fill function in the fragment shader
    pub(crate) fn in_params_str(self) -> String {
        let mut ret = "".to_string();

        for param in self.parameters {
            let index = param.index;
            let s = match param.param_type{
                ShaderParamType::F32 => format!(", bitcast<f32>(in.param_{index})"),
                ShaderParamType::U32 => format!(", in.param_{index}"),
            };
            ret.push_str(s.as_str());
        }

        return ret;
    }

    pub(crate) fn func_def_arguments(self) -> String {
        let mut ret = "".to_string();

        for param in self.parameters {
            let index = param.index;
            let t = match param.param_type{
                ShaderParamType::F32 => "f32",
                ShaderParamType::U32 => "u32",
            };
            ret.push_str(format!(",param_{index}: {t}").as_str())
        }

        return ret;
    }
}
