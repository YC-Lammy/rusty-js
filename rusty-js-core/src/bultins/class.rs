use std::collections::HashMap;
use std::sync::Arc;

use crate::{utils::nohasher, types::JValue};

use super::function::{JSFunction, JSFunctionInstance};

#[derive(Clone)]
pub struct JSClassInstance{
    class:Arc<JSClass>,
    constructor_instance:Option<JSFunctionInstance>,

}

pub struct JSClass {
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
    pub fn new() -> Self {
        Self {
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

    pub fn create_instance_with_capture(self:Arc<Self>, capture_stack:*mut JValue) -> JSClassInstance{
        let c = if let Some(v) = &self.constructor{
            Some(v.clone().create_instance_with_capture(None, capture_stack))
        } else{
            None
        };

        JSClassInstance { 
            class: self, 
            constructor_instance: c
        }
    }

    pub fn create_instance(self:Arc<Self>) -> JSClassInstance{
        let c = if let Some(v) = &self.constructor{
            Some(v.clone().create_instance(None))
        } else{
            None
        };

        JSClassInstance { 
            class: self, 
            constructor_instance: c
        }
    }
}
