
use crate::bultins::object::JObject;
use crate::bultins::strings::JSString;
use crate::error::Error;
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
        JValue::create_number(v)
    }
}

macro_rules! from_numeric {
    ($n:ty) => {
        impl From<$n> for JValue {
            fn from(v: $n) -> Self {
                (v as f64).into()
            }
        }
    };
}

from_numeric!(i32);
from_numeric!(i64);
from_numeric!(isize);
from_numeric!(u32);
from_numeric!(u64);
from_numeric!(usize);

impl From<String> for JValue {
    fn from(s: String) -> Self {
        Self::create_string(s.into())
    }
}

impl From<JObject> for JValue {
    fn from(obj: JObject) -> Self {
        JValue::create_object(obj)
    }
}

impl From<JSString> for JValue {
    fn from(s: JSString) -> Self {
        Self::create_string(s)
    }
}

impl From<Error> for JValue {
    fn from(e: Error) -> Self {
        JValue::UNDEFINED
    }
}
