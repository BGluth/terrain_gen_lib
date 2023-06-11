//! All passes are able to set a handful of parameters that the user can configure them with. These are just a set of key/value pairs.

pub type IntParamType = i64;
pub type FloatParamType = f64;

use std::collections::HashMap;

#[derive(Debug)]
pub(crate) enum ParamTypeAndOptDefault {
    String(Option<String>),
    Int(Option<IntParamType>),
    Float(Option<FloatParamType>),
}

pub(crate) enum PassParam {
    String(String),
    Int(IntParamType),
    Float(FloatParamType),
}

impl PassParam {
    pub fn get_string_param(&self, _p_name: &str) -> &str {
        todo!()
    }

    pub fn get_int_param(&self, _p_name: &str) -> IntParamType {
        todo!()
    }

    pub fn get_float_param(&self, _p_name: &str) -> FloatParamType {
        todo!()
    }
}

#[derive(Debug)]
pub(crate) struct PassParamReg {
    pub(crate) name: String,
    pub(crate) type_and_default: ParamTypeAndOptDefault,
}

struct PassParamRegIntern {
    default_val: ParamTypeAndOptDefault,
    explicit_val: Option<PassParam>,
}

struct PassParamsBuilder {
    reged_params: HashMap<String, PassParamRegIntern>,
}

impl FromIterator<PassParamReg> for PassParamsBuilder {
    fn from_iter<T: IntoIterator<Item = PassParamReg>>(_iter: T) -> Self {
        todo!()
    }
}

impl PassParamsBuilder {
    pub fn set_param(&mut self, _p_name: String, _val: PassParam) {
        todo!()
    }

    pub(crate) fn build(self) -> HashMap<String, PassParam> {
        todo!()
    }
}
