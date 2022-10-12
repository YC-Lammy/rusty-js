use super::object::JObject;

#[derive(Clone)]
pub struct Proxy {
    pub target: JObject,
    pub handler: JObject,
}
