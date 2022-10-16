use std::sync::Arc;

use crate::bultins::function::{JSFuncContext, JSFunction};
use crate::bultins::object::JObject;
use crate::bultins::strings::JString;
use crate::types::*;

impl From<bool> for JValue {
    fn from(b: bool) -> Self {
        if b {
            Self::TRUE
        } else {
            Self::FALSE
        }
    }
}

impl From<f64> for JValue {
    fn from(v: f64) -> Self {
        Self {
            value: JValueUnion { number: v },
            type_pointer: &JTypeVtable::NUMBER,
        }
    }
}

impl From<i64> for JValue {
    fn from(v: i64) -> Self {
        Self {
            value: JValueUnion { bigint: v },
            type_pointer: &JTypeVtable::BIGINT,
        }
    }
}

impl From<String> for JValue {
    fn from(s: String) -> Self {
        Self::String(JString::from(s))
    }
}

impl From<JObject> for JValue {
    fn from(obj: JObject) -> Self {
        JValue::Object(obj)
    }
}

impl From<JString> for JValue {
    fn from(s: JString) -> Self {
        Self::String(s)
    }
}

pub trait Convert<T> {
    fn convert(self) -> T;
}

impl<F> Convert<JSFunction> for F
where
    F: Fn(&JSFuncContext, JValue, &[JValue]) -> Result<JValue, JValue> + 'static,
{
    fn convert(self) -> JSFunction {
        JSFunction::Native(Arc::new(self))
    }
}
