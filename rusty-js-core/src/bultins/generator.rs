use std::ops::DerefMut;
use std::pin::Pin;
use std::sync::Arc;

use futures::lock::Mutex as FutureMutex;
use parking_lot::Mutex;

use crate::{baseline, Promise};
use crate::{error::Error, types::JValue, utils::string_interner::NAMES, JObject, Runtime};

use super::flag::PropFlag;

pub enum GeneratorResult {
    Await(JValue),
    Yield(JValue),
}

//#[cfg(nightly)]
#[derive(Clone)]
pub struct JSGenerator {
    pub(crate) is_async: bool,
    pub(crate) generator: Arc<Mutex<dyn Iterator<Item = Result<JValue, JValue>>>>,
    // stores done or not
    lock: Arc<FutureMutex<bool>>,
}

impl JSGenerator {
    pub fn resume(&self, mut value: JValue, runtime: &Runtime) -> Result<JValue, JValue> {
        if self.is_async {
            let lock = self.lock.clone();
            let generator = self.generator.clone();

            // create a future
            let f = async move {
                let lock_guard = lock.lock().await;

                if *lock_guard {
                    let obj = JObject::new();
                    obj.insert_property(NAMES["value"], JValue::UNDEFINED, PropFlag::THREE);
                    obj.insert_property(NAMES["done"], true.into(), PropFlag::THREE);
                    return Ok(obj.into());
                };

                let mut regs = [JValue::UNDEFINED; 3];

                let re = loop {
                    let runtime = Runtime::current();
                };

                drop(lock_guard);
                return re;
            };

            let p = runtime.to_mut().run_async(f);
            let obj = JObject::with_promise(p);

            return Ok(obj.into());
        } else {
            let value = self.generator.lock().next();

            match value {
                Some(v) => v,
                None => {
                    let obj = JObject::new();
                    obj.insert_property(NAMES["value"], JValue::UNDEFINED, PropFlag::THREE);
                    obj.insert_property(NAMES["done"], true.into(), PropFlag::THREE);
                    return Ok(obj.into());
                }
            }
        }
    }
}
