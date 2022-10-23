use crate::{types::JValue, Runtime};

use super::{object::JObject, prop::PropFlag};

pub struct ObjectBuilder {
    fields: Vec<(String, JValue)>,
    getters: Vec<(String, JObject)>,
    setters: Vec<(String, JObject)>,

    prototype: Option<JObject>,
}

impl ObjectBuilder {
    pub fn new() -> Self {
        Self {
            fields: Vec::new(),
            getters: Vec::new(),
            setters: Vec::new(),
            prototype: None,
        }
    }

    pub fn field<S, V>(mut self, name: S, value: V) -> Self
    where
        S: Into<String>,
        V: Into<JValue>,
    {
        self.fields.push((name.into(), value.into()));
        return self;
    }

    /// build must be called in a thread with runtime attached
    pub fn build(&self) -> JObject {
        let runtime = Runtime::current();

        let mut obj = JObject::new();
        for (name, value) in &self.fields {
            obj.insert_property(&name, *value, PropFlag::THREE);
        }

        for (name, value) in &self.getters {
            let id = runtime.register_field_name(&name);
            obj.bind_getter(id, *value);
        }

        for (name, value) in &self.setters {
            let id = runtime.register_field_name(&name);
            obj.bind_setter(id, *value);
        }

        if self.prototype.is_some() {}

        return obj;
    }
}
