use crate::{types::JValue, JObject};

pub trait CustomObject {
    fn get_property(&mut self, name: &str) -> Option<JValue>;
    fn set_property(&mut self, name: &str, value: JValue) -> bool;

    /// return true if callable
    fn is_callable(&self) -> bool;
    fn call(&mut self, this: JValue, args: &[JValue]) -> Result<JValue, JValue>;
    fn new(&mut self, new_target: JObject, args: &[JValue]) -> Result<JValue, JValue>;

    fn keys(&self) -> Vec<&str>;
    fn has_owned_property(&self, name: &str) -> bool {
        // slow implementation
        self.keys().contains(&name)
    }
}
