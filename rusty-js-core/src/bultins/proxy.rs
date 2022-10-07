use super::object::JObject;

pub struct Proxy {
    pub target: JObject,
    pub handler: JObject,
}
