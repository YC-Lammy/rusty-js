use std::collections::HashMap;
use std::sync::Arc;

use crate::{error::Error, types::JValue, utils::nohasher, JObject, Runtime};

use super::{
    function::{JSFunction, JSFunctionInstance},
    object::JObjectValue,
    prop::PropFlag,
};

#[derive(Clone)]
pub struct JSClassInstance {
    pub(crate) class: Arc<JSClass>,
    pub(crate) super_: Option<JObject>,
    pub(crate) constructor_instance: Option<JSFunctionInstance>,
}

impl JSClassInstance {
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
            return (JValue::Error(Error::ClassCannotBeInvokedWithoutNew), true);
        }
        // this must be a new target
        if !unsafe { this.value.object.is_new_target() } {
            return (JValue::Error(Error::ClassCannotBeInvokedWithoutNew), true);
        }

        let re = constructor.get_property_str("prototype");
        if re.is_err() {
            return (re.err().unwrap(), true);
        }

        let proto = re.unwrap();

        this.set_property_str("__proto__", proto).unwrap();

        if let Some(v) = &self.constructor_instance {
            let (v, err) = v.call(runtime, this, stack, argc);
            if err {
                return (v, true);
            }

            if v.is_object() {
                return (v, false);
            }

            return (this, false);
        } else {
            // no constructor provided
            // call on super()
            if let Some(s) = &self.super_ {
                let (v, err) = s.call(runtime, this, stack, argc as u32);

                if err {
                    return (v, err);
                }
            }

            return (this, false);
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
        unsafe { (self as *const Self as *mut Self).as_mut().unwrap() }
    }

    pub fn create_instance_with_capture(
        self: Arc<Self>,
        capture_stack: *mut JValue,
    ) -> JSClassInstance {
        let c = if let Some(v) = &self.constructor {
            Some(v.clone().create_instance_with_capture(None, capture_stack))
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
            Some(v.clone().create_instance(None))
        } else {
            None
        };

        JSClassInstance {
            class: self,
            super_: None,
            constructor_instance: c,
        }
    }

    pub fn create_object_with_capture(self: Arc<Self>, capture_stack: *mut JValue) -> JObject {
        let inst = self.clone().create_instance_with_capture(capture_stack);
        let mut obj = JObject::new();
        obj.inner.to_mut().wrapped_value = JObjectValue::Class(inst);

        let mut prototype = JObject::new();

        for key in &self.props {
            prototype.insert_property_static(
                *key,
                JValue::UNDEFINED,
                PropFlag::CONFIGURABLE | PropFlag::WRITABLE,
            );
        }

        for (k, f) in &self.methods {
            let f = f.clone().create_instance_with_capture(None, capture_stack);
            prototype.insert_property_static(
                *k,
                JObject::with_function(f).into(),
                PropFlag::CONFIGURABLE | PropFlag::WRITABLE,
            );
        }

        for (key, (getter, setter)) in &self.get_setters {
            if let Some(g) = getter {
                let f = g.clone().create_instance_with_capture(None, capture_stack);
                prototype.bind_getter(*key, JObject::with_function(f));
            }

            if let Some(s) = setter {
                let f = s.clone().create_instance_with_capture(None, capture_stack);
                prototype.bind_setter(*key, JObject::with_function(f));
            }
        }

        for key in &self.static_props {
            obj.insert_property_static(
                *key,
                JValue::UNDEFINED,
                PropFlag::CONFIGURABLE | PropFlag::WRITABLE,
            );
        }

        for (key, (getter, setter)) in &self.static_get_setters {
            if let Some(g) = getter {
                let f = g.clone().create_instance_with_capture(None, capture_stack);
                obj.bind_getter(*key, JObject::with_function(f));
            }

            if let Some(s) = setter {
                let f = s.clone().create_instance_with_capture(None, capture_stack);
                obj.bind_setter(*key, JObject::with_function(f));
            }
        }

        for (k, f) in &self.static_methods {
            let f = f.clone().create_instance_with_capture(None, capture_stack);
            obj.insert_property_static(
                *k,
                JObject::with_function(f).into(),
                PropFlag::CONFIGURABLE | PropFlag::WRITABLE,
            );
        }

        obj.insert_property("prototype", prototype.into(), PropFlag::NONE);
        obj.insert_property(
            "name",
            JValue::String(self.name.as_str().into()),
            PropFlag::CONFIGURABLE,
        );

        let len = if self.constructor.is_none() {
            0
        } else {
            self.constructor.as_ref().unwrap().args_len()
        };

        obj.insert_property("length", JValue::Number(len as f64), PropFlag::CONFIGURABLE);

        return obj;
    }

    pub fn create_object_without_capture(self: Arc<Self>) -> JObject {
        let inst = self.clone().create_instance();
        let mut obj = JObject::new();
        obj.inner.to_mut().wrapped_value = JObjectValue::Class(inst);

        let mut prototype = JObject::new();

        for key in &self.props {
            prototype.insert_property_static(
                *key,
                JValue::UNDEFINED,
                PropFlag::CONFIGURABLE | PropFlag::WRITABLE,
            );
        }

        for (k, f) in &self.methods {
            let f = f.clone().create_instance(None);
            prototype.insert_property_static(
                *k,
                JObject::with_function(f).into(),
                PropFlag::CONFIGURABLE | PropFlag::WRITABLE,
            );
        }

        for (key, (getter, setter)) in &self.get_setters {
            if let Some(g) = getter {
                let f = g.clone().create_instance(None);
                prototype.bind_getter(*key, JObject::with_function(f));
            }

            if let Some(s) = setter {
                let f = s.clone().create_instance(None);
                prototype.bind_setter(*key, JObject::with_function(f));
            }
        }

        for key in &self.static_props {
            obj.insert_property_static(
                *key,
                JValue::UNDEFINED,
                PropFlag::CONFIGURABLE | PropFlag::WRITABLE,
            );
        }

        for (key, (getter, setter)) in &self.static_get_setters {
            if let Some(g) = getter {
                let f = g.clone().create_instance(None);
                obj.bind_getter(*key, JObject::with_function(f));
            }

            if let Some(s) = setter {
                let f = s.clone().create_instance(None);
                obj.bind_setter(*key, JObject::with_function(f));
            }
        }

        for (k, f) in &self.static_methods {
            let f = f.clone().create_instance(None);
            obj.insert_property_static(
                *k,
                JObject::with_function(f).into(),
                PropFlag::CONFIGURABLE | PropFlag::WRITABLE,
            );
        }

        prototype.insert_property(
            "constructor",
            obj.into(),
            PropFlag::CONFIGURABLE | PropFlag::WRITABLE,
        );

        obj.insert_property("prototype", prototype.into(), PropFlag::NONE);
        obj.insert_property(
            "name",
            JValue::String(self.name.as_str().into()),
            PropFlag::CONFIGURABLE,
        );

        let len = if self.constructor.is_none() {
            0
        } else {
            self.constructor.as_ref().unwrap().args_len()
        };

        obj.insert_property("length", JValue::Number(len as f64), PropFlag::CONFIGURABLE);

        return obj;
    }
}
