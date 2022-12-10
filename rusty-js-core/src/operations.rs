use std::borrow::Cow;

use crate::bultins::function::CaptureStack;
use crate::bultins::object::JObject;
use crate::error::Error;
use crate::runtime::{FuncID, Runtime, TemplateID};
use crate::types::JValue;
use crate::utils::iterator::JSIterator;
use crate::utils::string_interner::NAMES;
use crate::{JSContext, PropKey, ToProperyKey};


#[repr(C)]
pub struct Result(pub JValue, pub bool);

impl Default for Result{
    fn default() -> Self {
        Self(JValue::UNDEFINED, false)
    }
}

#[no_mangle]
pub extern "C" fn add(l: JValue, r: JValue, stack: *mut JValue, runtime: &Runtime, re: &mut Result) {
    *re = match l.add(r, JSContext { stack, runtime }) {
        Ok(v) => Result(v, false),
        Err(e) => Result(e, true),
    };
}

#[no_mangle]
pub extern "C" fn sub(l: JValue, r: JValue, stack: *mut JValue, runtime: &Runtime, re: &mut Result) {
    *re = match l.sub(r, JSContext { stack, runtime }) {
        Ok(v) => Result(v, false),
        Err(e) => Result(e, true),
    };
}

#[no_mangle]
pub extern "C" fn mul(l: JValue, r: JValue, stack: *mut JValue, runtime: &Runtime, re: &mut Result) {
    *re = match l.mul(r, JSContext { stack, runtime }) {
        Ok(v) => Result(v, false),
        Err(e) => Result(e, true),
    };
}

#[no_mangle]
pub extern "C" fn div(l: JValue, r: JValue, stack: *mut JValue, runtime: &Runtime, re: &mut Result) {
    *re = match l.div(r, JSContext { stack, runtime }) {
        Ok(v) => Result(v, false),
        Err(e) => Result(e, true),
    };
}

#[no_mangle]
pub extern "C" fn rem(l: JValue, r: JValue, stack: *mut JValue, runtime: &Runtime, re: &mut Result) {
    *re = match l.rem(r, JSContext { stack, runtime }) {
        Ok(v) => Result(v, false),
        Err(e) => Result(e, true),
    };
}

#[no_mangle]
pub extern "C" fn pow(l: JValue, r: JValue, stack: *mut JValue, runtime: &Runtime, re: &mut Result) {
    *re = match l.exp(r, JSContext { stack, runtime }) {
        Ok(v) => Result(v, false),
        Err(e) => Result(e, true),
    };
}

#[no_mangle]
pub extern "C" fn shr(l: JValue, r: JValue, stack: *mut JValue, runtime: &Runtime, re: &mut Result) {
    *re = match l.rshift(r, JSContext { stack, runtime }) {
        Ok(v) => Result(v, false),
        Err(e) => Result(e, true),
    };
}

#[no_mangle]
pub extern "C" fn zshr(l: JValue, r: JValue, stack: *mut JValue, runtime: &Runtime, re: &mut Result) {
    *re = match l.unsigned_rshift(r, JSContext { stack, runtime }) {
        Ok(v) => Result(v, false),
        Err(e) => Result(e, true),
    };
}

#[no_mangle]
pub extern "C" fn shl(l: JValue, r: JValue, stack: *mut JValue, runtime: &Runtime, re: &mut Result) {
    *re = match l.lshift(r, JSContext { stack, runtime }) {
        Ok(v) => Result(v, false),
        Err(e) => Result(e, true),
    };
}

#[inline]
#[no_mangle]
pub extern "C" fn eqeq(l: JValue, r: JValue, stack: *mut JValue, runtime: &Runtime, re: &mut Result) {
    *re = match l.eqeq(r, JSContext { stack, runtime }) {
        Ok(v) => Result(v.into(), false),
        Err(e) => Result(e, true),
    };
}

#[no_mangle]
pub extern "C" fn lt(l: JValue, r: JValue, stack: *mut JValue, runtime: &Runtime, re: &mut Result) {
    let l = match l.to_number(JSContext { stack, runtime }) {
        Ok(v) => v,
        Err(e) => {
            *re = Result(e, true);
            return
        },
    };
    let r = match r.to_number(JSContext { stack, runtime }) {
        Ok(v) => v,
        Err(e) => {
            *re = Result(e, true);
            return
        },
    };

    *re = Result((l < r).into(), false);
}

#[no_mangle]
pub extern "C" fn lteq(l: JValue, r: JValue, stack: *mut JValue, runtime: &Runtime, re: &mut Result) {
    let l = match l.to_number(JSContext { stack, runtime }) {
        Ok(v) => v,
        Err(e) => { *re = Result(e, true); return;},
    };
    let r = match r.to_number(JSContext { stack, runtime }) {
        Ok(v) => v,
        Err(e) => { *re = Result(e, true); return;},
    };

    *re = Result((l <= r).into(), false);
}

#[no_mangle]
pub extern "C" fn gt(l: JValue, r: JValue, stack: *mut JValue, runtime: &Runtime, re: &mut Result) {
    let l = match l.to_number(JSContext { stack, runtime }) {
        Ok(v) => v,
        Err(e) => { *re = Result(e, true); return;},
    };
    let r = match r.to_number(JSContext { stack, runtime }) {
        Ok(v) => v,
        Err(e) => { *re = Result(e, true); return;},
    };

    *re = Result((l > r).into(), false);
}

#[no_mangle]
pub extern "C" fn gteq(l: JValue, r: JValue, stack: *mut JValue, runtime: &Runtime, re: &mut Result) {
    let l = match l.to_number(JSContext { stack, runtime }) {
        Ok(v) => v,
        Err(e) => { *re = Result(e, true); return;},
    };
    let r = match r.to_number(JSContext { stack, runtime }) {
        Ok(v) => v,
        Err(e) => { *re = Result(e, true); return;},
    };

    *re = Result((l >= r).into(), false);
}

#[no_mangle]
pub extern "C" fn dynamic_get(runtime: &Runtime, key: u32, re: &mut Result) {
    *re = match runtime.to_mut().get_variable(key) {
        Ok(v) => Result(v, false),
        Err(e) => Result(e, true),
    };
}

#[no_mangle]
pub extern "C" fn dynamic_set(runtime: &Runtime, key: u32, value: JValue, re: &mut Result) {
    *re = match runtime.to_mut().set_variable(key, value) {
        Ok(()) => Result(JValue::UNDEFINED, false),
        Err(e) => Result(e, true),
    };
}

#[no_mangle]
pub extern "C" fn call(
    func: JValue,
    runtime: &Runtime,
    this: JValue,
    stack: *mut JValue,
    argc: usize,
    re: &mut Result
) {
    *re = if let Some(obj) = func.as_object() {
        let (v, err) = obj.call(runtime, this, stack, argc);
        Result(v, err)
    } else {
        Result(Error::CallOnNonFunction.into(), true)
    };
}

#[no_mangle]
pub extern "C" fn invoke_new(
    constructor: JValue,
    runtime: &Runtime,
    stack: *mut JValue,
    argc: usize,
    result: &mut Result) {
    if constructor.is_object() {
        let this = JObject::new_target();
        let proto = constructor.get_property("prototype", crate::JSContext { stack, runtime });
        let proto = match proto {
            Ok(v) => v,
            Err(e) => { *result = Result(e, true); return;},
        };
        this.insert_property(NAMES["__proto__"], proto, Default::default());

        // get the old target
        let old_target = runtime.new_target;
        runtime.to_mut().new_target = constructor;

        let re = unsafe {
            let args = std::slice::from_raw_parts(stack, argc);
            constructor.call(
                this.into(),
                args,
                JSContext {
                    stack: stack.add(argc),
                    runtime,
                },
            )
        };

        // remove the new tag
        if this.is_new_target() {
            this.inner.to_mut().wrapped_value = Default::default();
        }
        // set to the old target
        runtime.to_mut().new_target = old_target;

        let v = match re {
            Ok(v) => v,
            Err(e) => { *result = Result(e, true); return;},
        };

        if v.is_object() {
            *result = Result(v, false);
            return;
        }
        *result = Result(this.into(), false);
    } else {
        *result = Result(
            JValue::from(Error::TypeError(
                "calling new on non constructor".to_string(),
            )),
            true,
        );
    }
}

#[no_mangle]
pub extern "C" fn new_target(runtime: &Runtime) -> JValue {
    return runtime.new_target;
}

#[no_mangle]
pub extern "C" fn import_meta(runtime: &Runtime) -> JValue {
    return runtime.import_meta;
}

pub unsafe fn spread(
    value: JValue,
    runtime: &Runtime,
    stack: *mut JValue,
) -> (*mut JValue, usize, JValue, bool) {
    let iter = JSIterator::new(
        value,
        JSContext {
            stack: stack,
            runtime,
        },
    );
    let iter = match iter {
        Ok(v) => v,
        Err(e) => return (std::ptr::null_mut(), 0, e, true),
    };

    let mut values = Vec::new();
    for re in iter {
        match re {
            Ok(v) => values.push(v),
            Err(e) => return (std::ptr::null_mut(), 0, e, true),
        }
    }

    let v = values.leak();
    return (v.as_mut_ptr(), v.len(), JValue::UNDEFINED, false);
}

pub unsafe fn extend_object(obj: JValue, target: JValue, runtime: &Runtime) {
    assert!(obj.is_object());
    let obj = obj.as_object().unwrap();

    if let Some(s) = target.as_string() {
        let mut i = 0;
        for c in s.as_str().chars() {
            let key = i.to_string();
            let key = key.to_key(runtime);
            obj.insert_property(
                key,
                JValue::create_string(c.to_string().into()),
                Default::default(),
            );
            i += 1;
        }
    } else if let Some(target) = target.as_object() {
        obj.inner.to_mut().values.extend(&target.inner.values);
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
                .push((Default::default(), JValue::create_string(i.as_str().into())))
        }
    }

    let mut exprs = Vec::new();
    for i in args {
        if let Some(s) = i.as_string() {
            exprs.push(Cow::Owned(s.to_string()));
        } else {
            exprs.push(Cow::Owned(ToString::to_string(i)))
        };
    }
    tpl.create(&exprs)
}

pub unsafe fn create_function(id: u32, capture_stack: *mut CaptureStack) -> JValue {
    let cap = (&mut *capture_stack).clone();
    let runtime = Runtime::current();
    let func = runtime.get_function(FuncID(id)).unwrap();
    let ins = func.create_instance_with_capture(None, cap);
    JObject::with_function(ins).into()
}

pub unsafe fn bind_class_super(
    runtime: &Runtime,
    stack: *mut JValue,
    class: JValue,
    super_class: JValue,
    re: &mut Result) {
    // c must be a class object
    assert!(class.is_object());

    if !super_class.is_object() {
        *re = Result(JValue::from(Error::ClassExtendsNonCallable), true);
        return;
    }

    let class = class.as_object().unwrap();
    let super_class = super_class.as_object().unwrap();

    let proto = class
        .get_property(NAMES["prototype"], JSContext { stack, runtime })
        .unwrap();
    let super_proto = super_class
        .get_property(NAMES["prototype"], JSContext { stack, runtime })
        .unwrap();
    proto
        .set_property(
            NAMES["__proto__"],
            super_proto,
            JSContext { stack, runtime },
        )
        .unwrap();

    if !super_class.is_function_instance() || !super_class.is_class() {
        *re = Result(JValue::from(Error::ClassExtendsNonCallable), true);
        return
    }

    if let Some(c) = class.as_class() {
        c.set_super(super_class);
    }

    *re = Result(JValue::UNDEFINED, false);
}

pub unsafe fn super_prop(
    runtime: &Runtime,
    constructor: JValue,
    propname: JValue,
    stack: *mut JValue,
    re: &mut Result) {
    let prop = if let Some(s) = propname.as_string() {
        runtime.register_field_name(s.as_str())
    } else {
        runtime.register_field_name(&propname.to_string())
    };
    super_prop_static(runtime, constructor, prop, stack, re)
}

pub unsafe fn super_prop_static(
    runtime: &Runtime,
    constructor: JValue,
    prop: u32,
    stack: *mut JValue,
    re: &mut Result) {
    *re = if !runtime.new_target.is_undefined() && runtime.new_target == constructor {
        match constructor.get_property(PropKey(prop), JSContext { stack, runtime }) {
            Ok(v) => Result(v, false),
            Err(e) => Result(e, true),
        }
    } else if constructor.is_object() {
        let proto = constructor
            .get_property(NAMES["prototype"], JSContext { stack, runtime })
            .unwrap();
        match proto.get_property(PropKey(prop), JSContext { stack, runtime }) {
            Ok(v) => Result(v, false),
            Err(e) => Result(e, true),
        }
    } else {
        Result(JValue::UNDEFINED, false)
    };
}

pub unsafe fn super_write_prop(
    runtime: &Runtime,
    constructor: JValue,
    propname: JValue,
    value: JValue,
    stack: *mut JValue,
    re: &mut Result) {
    let prop = if let Some(s) = propname.as_string() {
        runtime.register_field_name(s.as_str())
    } else {
        runtime.register_field_name(&propname.to_string())
    };
    super_write_prop_static(runtime, constructor, prop, value, stack, re)
}

pub unsafe fn super_write_prop_static(
    runtime: &Runtime,
    constructor: JValue,
    prop: u32,
    value: JValue,
    stack: *mut JValue,
    re: &mut Result) {
    if !runtime.new_target.is_undefined() && runtime.new_target == constructor {
        match constructor.set_property(PropKey(prop), value, JSContext { stack, runtime }) {
            Ok(()) => {}
            Err(e) => { *re = Result(e, true); return;},
        }
    } else if constructor.is_object() {
        let proto = constructor
            .get_property(NAMES["prototype"], JSContext { stack, runtime })
            .unwrap();

        match proto.set_property(PropKey(prop), value, JSContext { stack, runtime }) {
            Ok(()) => {}
            Err(e) => { *re = Result(e, true); return;},
        }
    }

    *re = Result(JValue::UNDEFINED, false);
}
