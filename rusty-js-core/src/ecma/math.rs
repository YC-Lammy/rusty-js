use crate::{JObject, Runtime};

pub fn init(rt: &Runtime) -> JObject {
    let obj = rt.create_object().into();

    return obj;
}
