use crate::bultins::prop::PropFlag;
use crate::bultins::{self, JObject};
use crate::Runtime;

mod object;
mod number;

pub fn enable(runtime: &Runtime) {
    let obj = object::create_object(runtime);

    runtime.global_this.insert_property("Object", obj.into(), PropFlag::BUILTIN);

    let obj = number::creat_object(runtime);
    runtime.global_this.insert_property_builtin("Number", obj.into());
}