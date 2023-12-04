use std::fmt::Debug;

use bevy::reflect::Reflect;

/// Indicates which parameters are used by the function.
/// For example, if this is the sdf ParamUsage and bit 3 is set, then param_3 will be sent to that function.
/// Params are sent to the function in
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Reflect)]
pub struct ShaderParamUsage(u32);

impl ShaderParamUsage {
    /// Use no parameters
    pub const NO_PARAMS: Self = Self(0);

    /// Returns a copy which uses the param at `index`
    #[must_use]
    pub const fn with_param(self, index: u32) -> Self {
        //todo check at compile time that this is valid
        let mask = 1 << index;
        Self(self.0 | mask)
    }

    /// Returns a copy with uses all parameters
    pub const fn all_params<const PARAMS: usize> ()-> Self{
        let inner = u32::MAX.wrapping_shr(u32::BITS - (PARAMS as u32));
        Self(inner)
    }

    /// Returns a copy which uses the param at the given `indices`
    #[must_use]
    pub const fn from_params(indices: &[u32]) -> Self {
        //todo check at compile time that this is valid
        let mut result = Self::NO_PARAMS;
        let mut array_index = 0;
        loop {
            if array_index >= indices.len() {
                break;
            }

            let index = indices[array_index];
            result = result.with_param(index);
            array_index += 1;
        }

        result
    }

    // /// Returns whether the param at `index` is used
    // const fn uses_param(self, index: u32) -> bool {
    //     let mask = 1 << index;
    //     (self.0 & mask) != 0
    // }

    /// Creates the params string to be passed to the sdf or fill function in the fragment shader
    pub(crate) fn in_params_str(self) -> String {
        let mut ret = "".to_string();

        for param in ParamsIter::new(self) {
            ret.push_str(format!(", bitcast<f32>(in.param_{param})").as_str())
        }

        return ret;
    }

    pub(crate) fn func_def_arguments(self) -> String {
        let mut ret = "".to_string();

        for param in ParamsIter::new(self) {
            ret.push_str(format!(",param_{param}: f32").as_str())
        }

        return ret;
    }
}

#[derive(Copy, Clone, Debug)]
struct ParamsIter {
    inner: u32,
    done: u32,
}

impl Iterator for ParamsIter {
    type Item = u32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.inner == 0 {
            return None;
        }
        let zeros = self.inner.trailing_zeros();
        self.inner = self.inner.wrapping_shr(zeros + 1);
        let ret = self.done + zeros;
        self.done += zeros + 1;
        Some(ret)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let size = self.inner.count_zeros() as usize;
        (size, Some(size))
    }
}

impl ParamsIter {
    pub fn new(params_usage: ShaderParamUsage) -> Self {
        Self {
            inner: params_usage.0,
            done: 0,
        }
    }
}
