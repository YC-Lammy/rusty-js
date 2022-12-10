use crate::JValue;

use super::{object::JObjectInner, function::CaptureStack};



#[repr(C)]
pub struct JSFunctionObject{
    pub(crate) obj:JObjectInner,

    /// only used when is arrow
    pub(crate) this: JValue,
    pub(crate) capture_stack:CaptureStack,

    func:&'static super::function::JSFunction
}