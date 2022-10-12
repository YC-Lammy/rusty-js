use crate::types::JValue;

use super::object::JObject;



pub struct ObjectBuilder{
    fields:Vec<(String, JValue)>,
    getters:Vec<(String, JObject)>,
    setters:Vec<(String, JObject)>,

    prototype:Option<JObject>,
}

impl ObjectBuilder{
    pub fn new() -> Self{
        Self { 
            fields: Vec::new(), 
            getters: Vec::new(), 
            setters: Vec::new(), 
            prototype: None
        }
    }
}