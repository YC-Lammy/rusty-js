use std::collections::HashMap;
use std::sync::Arc;

use crate::{
    error::Error,
    operations,
    value::JValue,
    utils::{nohasher, string_interner::NAMES},
    JObject, JSContext, PropKey, Runtime,
};

use super::{
    object_property::PropFlag,
    function::{CaptureStack, JSFunction, JSFunctionInstance},
    object::JObjectValue,
};

pub struct JSClassInstance {
    pub(crate) class: Arc<JSClass>,
    pub(crate) super_: Option<JObject>,
    pub(crate) constructor_instance: Option<Arc<JSFunctionInstance>>,
}

impl Clone for JSClassInstance {
    fn clone(&self) -> Self {
        Self {
            class: self.class.clone(),
            super_: self.super_.clone(),
            constructor_instance: self.constructor_instance.clone(),
        }
    }
}

impl JSClassInstance {
    pub fn set_super(&self, super_: JObject) {
        unsafe { (&mut *(self as *const Self as *mut Self)).super_ = Some(super_) };
    }
    pub fn call(
        &self,
        runtime: &Runtime,
        constructor: JValue,
        this: JValue,
        stack: *mut JValue,
        argc: usize,
    ) -> (JValue, bool) {
        // new: this must be an object
        if !this.is_object() {
            return (JValue::from(Error::ClassCannotBeInvokedWithoutNew), true);
        }

        let this = this.as_object().unwrap();
        // this must be a new target
        if !unsafe { operations::new_target(runtime).is_object() } {
            return (JValue::from(Error::ClassCannotBeInvokedWithoutNew), true);
        }

        let re = constructor.get_property("prototype", JSContext { stack, runtime });
        if re.is_err() {
            return (re.err().unwrap(), true);
        }

        let proto = re.unwrap();

        this.set_property(
            "__proto__",
            proto,
            JSContext {
                stack: stack,
                runtime: runtime,
            },
        );

        if let Some(v) = &self.constructor_instance {
            let (v, err) = v.clone().call(runtime, this.into(), stack, argc);
            if err {
                return (v, true);
            }

            if v.is_object() {
                return (v, false);
            }

            return (this.into(), false);
        } else {
            // no constructor provided
            // call on super()
            if let Some(s) = &self.super_ {
                let (v, err) = s.call(runtime, this.into(), stack, argc);

                if err {
                    return (v, err);
                }
            }

            return (this.into(), false);
        }
    }
}

pub struct JSClass {
    pub(crate) name: String,
    pub(crate) constructor: Option<Arc<JSFunction>>,
    pub(crate) methods: HashMap<u32, Arc<JSFunction>, nohasher::NoHasherBuilder>,
    pub(crate) static_methods: HashMap<u32, Arc<JSFunction>, nohasher::NoHasherBuilder>,

    pub(crate) get_setters:
        HashMap<u32, (Option<Arc<JSFunction>>, Option<Arc<JSFunction>>), nohasher::NoHasherBuilder>,
    pub(crate) static_get_setters:
        HashMap<u32, (Option<Arc<JSFunction>>, Option<Arc<JSFunction>>), nohasher::NoHasherBuilder>,

    pub(crate) props: Vec<u32>,
    pub(crate) static_props: Vec<u32>,
}

impl JSClass {
    pub fn new(name: String) -> Self {
        Self {
            name,
            constructor: None,
            methods: Default::default(),
            static_methods: Default::default(),
            get_setters: Default::default(),
            static_get_setters: Default::default(),

            props: Vec::new(),
            static_props: Vec::new(),
        }
    }

    pub fn to_mut(&self) -> &mut Self {
        unsafe { &mut *(self as *const Self as *mut Self) }
    }

    pub fn create_instance_with_capture(
        self: Arc<Self>,
        capture_stack: CaptureStack,
    ) -> JSClassInstance {
        let c = if let Some(v) = &self.constructor {
            Some(Arc::new(
                v.clone().create_instance_with_capture(None, capture_stack),
            ))
        } else {
            None
        };

        JSClassInstance {
            class: self,
            super_: None,
            constructor_instance: c,
        }
    }

    pub fn create_instance(self: Arc<Self>) -> JSClassInstance {
        let c = if let Some(v) = &self.constructor {
            Some(Arc::new(v.clone().create_instance(None)))
        } else {
            None
        };

        JSClassInstance {
            class: self,
            super_: None,
            constructor_instance: c,
        }
    }

    pub fn create_with_capture(self: Arc<Self>, capture_stack: CaptureStack) -> JObject {
        let inst = self
            .clone()
            .create_instance_with_capture(capture_stack.clone());
        let obj = JObject::new();
        obj.inner.to_mut().wrapped_value = JObjectValue::Class(Arc::new(inst));

        let prototype = JObject::new();

        for key in &self.props {
            prototype.insert_property(
                PropKey(*key),
                JValue::UNDEFINED,
                PropFlag::CONFIGURABLE | PropFlag::WRITABLE,
            );
        }

        for (k, f) in &self.methods {
            let f = f
                .clone()
                .create_instance_with_capture(None, capture_stack.clone());
            prototype.insert_property(
                PropKey(*k),
                JObject::with_function(f).into(),
                PropFlag::CONFIGURABLE | PropFlag::WRITABLE,
            );
        }

        for (key, (getter, setter)) in &self.get_setters {
            if let Some(g) = getter {
                let f = g
                    .clone()
                    .create_instance_with_capture(None, capture_stack.clone());
                prototype.bind_getter(PropKey(*key), JObject::with_function(f));
            }

            if let Some(s) = setter {
                let f = s
                    .clone()
                    .create_instance_with_capture(None, capture_stack.clone());
                prototype.bind_setter(PropKey(*key), JObject::with_function(f));
            }
        }

        for key in &self.static_props {
            obj.insert_property(
                PropKey(*key),
                JValue::UNDEFINED,
                PropFlag::CONFIGURABLE | PropFlag::WRITABLE,
            );
        }

        for (key, (getter, setter)) in &self.static_get_setters {
            if let Some(g) = getter {
                let f = g
                    .clone()
                    .create_instance_with_capture(None, capture_stack.clone());
                obj.bind_getter(PropKey(*key), JObject::with_function(f));
            }

            if let Some(s) = setter {
                let f = s
                    .clone()
                    .create_instance_with_capture(None, capture_stack.clone());
                obj.bind_setter(PropKey(*key), JObject::with_function(f));
            }
        }

        for (k, f) in &self.static_methods {
            let f = f
                .clone()
                .create_instance_with_capture(None, capture_stack.clone());
            obj.insert_property(
                PropKey(*k),
                JObject::with_function(f).into(),
                PropFlag::CONFIGURABLE | PropFlag::WRITABLE,
            );
        }

        obj.insert_property(NAMES["prototype"], prototype.into(), PropFlag::NONE);
        obj.insert_property(
            NAMES["name"],
            JValue::create_string(self.name.as_str().into()),
            PropFlag::CONFIGURABLE,
        );

        let len = if self.constructor.is_none() {
            0
        } else {
            self.constructor.as_ref().unwrap().args_len
        };

        obj.insert_property(
            NAMES["length"],
            JValue::create_number(len as f64),
            PropFlag::CONFIGURABLE,
        );

        return obj;
    }

    pub fn create_without_capture(self: Arc<Self>) -> JObject {
        let inst = self.clone().create_instance();
        let obj = JObject::new();
        obj.inner.to_mut().wrapped_value = JObjectValue::Class(Arc::new(inst));

        let prototype = JObject::new();

        for key in &self.props {
            prototype.insert_property(
                PropKey(*key),
                JValue::UNDEFINED,
                PropFlag::CONFIGURABLE | PropFlag::WRITABLE,
            );
        }

        for (k, f) in &self.methods {
            let f = f.clone().create_instance(None);
            prototype.insert_property(
                PropKey(*k),
                JObject::with_function(f).into(),
                PropFlag::CONFIGURABLE | PropFlag::WRITABLE,
            );
        }

        for (key, (getter, setter)) in &self.get_setters {
            if let Some(g) = getter {
                let f = g.clone().create_instance(None);
                prototype.bind_getter(PropKey(*key), JObject::with_function(f));
            }

            if let Some(s) = setter {
                let f = s.clone().create_instance(None);
                prototype.bind_setter(PropKey(*key), JObject::with_function(f));
            }
        }

        for key in &self.static_props {
            obj.insert_property(
                PropKey(*key),
                JValue::UNDEFINED,
                PropFlag::CONFIGURABLE | PropFlag::WRITABLE,
            );
        }

        for (key, (getter, setter)) in &self.static_get_setters {
            if let Some(g) = getter {
                let f = g.clone().create_instance(None);
                obj.bind_getter(PropKey(*key), JObject::with_function(f));
            }

            if let Some(s) = setter {
                let f = s.clone().create_instance(None);
                obj.bind_setter(PropKey(*key), JObject::with_function(f));
            }
        }

        for (k, f) in &self.static_methods {
            let f = f.clone().create_instance(None);
            obj.insert_property(
                PropKey(*k),
                JObject::with_function(f).into(),
                PropFlag::CONFIGURABLE | PropFlag::WRITABLE,
            );
        }

        prototype.insert_property(
            NAMES["constructor"],
            obj.into(),
            PropFlag::CONFIGURABLE | PropFlag::WRITABLE,
        );

        obj.insert_property(NAMES["prototype"], prototype.into(), PropFlag::NONE);
        obj.insert_property(
            NAMES["name"],
            JValue::create_string(self.name.as_str().into()),
            PropFlag::CONFIGURABLE,
        );

        let len = if self.constructor.is_none() {
            0
        } else {
            self.constructor.as_ref().unwrap().args_len
        };

        obj.insert_property(
            NAMES["length"],
            JValue::create_number(len as f64),
            PropFlag::CONFIGURABLE,
        );

        return obj;
    }
}
