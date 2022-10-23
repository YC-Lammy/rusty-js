use std::borrow::Cow;

use crate::bultins::object::JObject;
use crate::bultins::promise::Promise;
use crate::error::Error;
use crate::fast_iter::FastIterator;
use crate::runtime::{AsyncResult, FuncID, Runtime, TemplateID};
use crate::types::JValue;

#[inline]
pub fn invoke_new(
    constructor: JValue,
    runtime: &Runtime,
    stack: *mut JValue,
    argc: u32,
) -> (JValue, bool) {
    if constructor.is_object() {
        let mut this = JObject::new_target();
        let proto = constructor.get_property_str("prototype");
        let proto = match proto {
            Ok(v) => v,
            Err(e) => return (e, true),
        };
        this.insert_property("__proto__", proto, Default::default());

        // get the old target
        let old_target = runtime.new_target;
        runtime.to_mut().new_target = constructor;

        let (v, err) = unsafe {
            constructor
                .value
                .object
                .call(runtime, this.into(), stack, argc)
        };

        // remove the new tag
        if this.is_new_target() {
            this.inner.to_mut().wrapped_value = Default::default();
        }
        // set to the old target
        runtime.to_mut().new_target = old_target;

        if err {
            return (v, true);
        }
        if v.is_object() {
            return (v, false);
        }
        return (this.into(), false);
    } else {
        return (
            JValue::Error(Error::TypeError(
                "calling new on non constructor".to_string(),
            )),
            true,
        );
    }
}

pub fn new_target(runtime: &Runtime) -> JValue {
    return runtime.new_target;
}

pub fn import_meta(runtime: &Runtime) -> JValue {
    return runtime.import_meta;
}

pub fn async_wait(value: JValue) -> (JValue, bool) {
    if let Some(p) = value.as_promise() {
        match p {
            Promise::Fulfilled(v) => (*v, false),
            Promise::Rejected(v) => (*v, true),
            Promise::Pending { id } => {
                let runtime = Runtime::current();

                loop {
                    let re = runtime.to_mut().poll_async(*id, JValue::UNDEFINED);
                    match re {
                        AsyncResult::Err(e) => return (e, true),
                        AsyncResult::Return(r) => return (r, false),
                        // ignore yield value
                        AsyncResult::Yield(_) => {
                            // suspend execution
                            runtime.async_executor.suspend(JValue::UNDEFINED);
                        }
                    }
                }
            }
        }
    } else {
        return (value, false);
    }
}

pub fn Yield(value: JValue) -> JValue {
    let runtime = Runtime::current();
    runtime.generator_executor.suspend(value)
}

pub unsafe fn spread(value: JValue, this: JValue, stack: *mut JValue) -> (*mut JValue, u64, bool) {
    let iter = FastIterator::new(value, crate::bytecodes::LoopHint::For);

    let mut values = Vec::new();
    loop {
        let (done, error, value) = iter.next(this, stack);

        if error {
            return (Box::leak(Box::new(value)), 1, true);
        }

        values.push(value);

        if done {
            FastIterator::drop_(iter);
            break;
        }
    }

    let mut v = Vec::with_capacity(values.len());
    v.extend_from_slice(&values);
    let v = v.leak();

    return (v.as_mut_ptr(), v.len() as u64, false);
}

pub unsafe fn extend_object(obj: JValue, target: JValue) {
    assert!(obj.is_object());
    let mut obj = obj.value.object;

    if target.is_string() {
        let mut i = 0;
        for c in target.value.string.as_str().chars() {
            obj.insert_property(
                &i.to_string(),
                JValue::String(c.to_string().into()),
                Default::default(),
            );
            i += 1;
        }
    } else if target.is_object() {
        obj.inner
            .to_mut()
            .values
            .extend(&target.value.object.inner.values);
    };
}

pub unsafe fn create_template(id: u32, args: *mut JValue, argc: u32, tagged: bool) -> JValue {
    let args = std::slice::from_raw_parts_mut(args, argc as usize);
    let runtime = Runtime::current();
    let tpl = runtime.get_template(TemplateID(id));

    if tagged {
        let array = JObject::array();
        for i in &tpl.strings {
            array
                .as_array()
                .unwrap()
                .push((Default::default(), JValue::String(i.as_str().into())))
        }
    }

    let mut exprs = Vec::new();
    for i in args {
        if i.is_string() {
            exprs.push(Cow::Borrowed(i.value.string.as_ref()));
        } else {
            exprs.push(Cow::Owned(i.to_string()))
        };
    }
    tpl.create(&exprs)
}

pub unsafe fn create_function(id: u32, capture_stack: *mut JValue) -> JValue {
    let runtime = Runtime::current();
    let func = runtime.get_function(FuncID(id)).unwrap();
    let ins = func.create_instance_with_capture(None, capture_stack);
    JObject::with_function(ins).into()
}

pub unsafe fn bind_class_super(c: JValue, super_class: JValue) -> (JValue, bool) {
    // c must be a class object
    assert!(c.is_object());

    let proto = c.get_property_str("prototype").unwrap();
    let super_proto = super_class.get_property_str("prototype").unwrap();
    proto.set_property_str("__proto__", super_proto).unwrap();

    if !super_class.is_object() {
        return (JValue::Error(Error::ClassExtendsNonCallable), true);
    }

    if !super_class.value.object.is_function_instance() || !super_class.value.object.is_class() {
        return (JValue::Error(Error::ClassExtendsNonCallable), true);
    }

    if let Some(c) = c.value.object.as_class() {
        c.super_ = Some(super_class.value.object)
    }

    return (JValue::UNDEFINED, false);
}

pub unsafe fn super_prop(
    runtime: &Runtime,
    constructor: JValue,
    propname: JValue,
    stack: *mut JValue,
) -> (JValue, bool) {
    let prop = if propname.is_string() {
        runtime.register_field_name(propname.value.string.as_str())
    } else {
        runtime.register_field_name(&propname.to_string())
    };
    super_prop_static(runtime, constructor, prop, stack)
}

pub unsafe fn super_prop_static(
    runtime: &Runtime,
    constructor: JValue,
    prop: u32,
    stack: *mut JValue,
) -> (JValue, bool) {
    if !runtime.new_target.is_undefined() && runtime.new_target == constructor {
        return constructor.get_property_raw(prop, stack);
    } else if constructor.is_object() {
        let proto = constructor.get_property_str("prototype").unwrap();
        return proto.get_property_raw(prop, stack);
    }

    return (JValue::UNDEFINED, false);
}

pub unsafe fn super_write_prop(
    runtime: &Runtime,
    constructor: JValue,
    propname: JValue,
    value: JValue,
    stack: *mut JValue,
) -> (JValue, bool) {
    let prop = if propname.is_string() {
        runtime.register_field_name(propname.value.string.as_str())
    } else {
        runtime.register_field_name(&propname.to_string())
    };
    super_write_prop_static(runtime, constructor, prop, value, stack)
}

pub unsafe fn super_write_prop_static(
    runtime: &Runtime,
    constructor: JValue,
    prop: u32,
    value: JValue,
    stack: *mut JValue,
) -> (JValue, bool) {
    if !runtime.new_target.is_undefined() && runtime.new_target == constructor {
        return constructor.set_property_raw(prop, value, stack);
    } else if constructor.is_object() {
        let proto = constructor.get_property_str("prototype").unwrap();
        return proto.set_property_raw(prop, value, stack);
    }

    return (JValue::UNDEFINED, false);
}
