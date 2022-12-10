use crate::bultins::flag::PropFlag;
use crate::utils::string_interner::NAMES;
use crate::Runtime;

mod array;
mod math;
mod number;
mod object;

pub fn enable(runtime: &Runtime) {
    let obj = object::ect(runtime);

    runtime
        .global_this
        .insert_property(NAMES["Object"], obj.into(), PropFlag::BUILTIN);

    let obj = number::creat_object(runtime);
    runtime
        .global_this
        .insert_property_builtin(NAMES["Number"], obj.into());

    let obj = array::init(runtime);
    runtime
        .global_this
        .insert_property_builtin(NAMES["Array"], obj.into());

    let obj = math::init(runtime);
    runtime
        .global_this
        .insert_property_builtin(NAMES["Math"], obj.into());
}
