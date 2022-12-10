use std::collections::HashMap;

use crate::{JObject, JValue, Runtime};

use super::GcFlag;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct FinalizationRegistryId(pub(crate) u32);

#[derive(Default)]
pub struct FinalizeRegistry {
    values: HashMap<FinalizationRegistryId, (JObject, Vec<(JObject, JValue)>)>,
}

impl FinalizeRegistry {
    pub unsafe fn trace(&self) {
        self.values.iter().for_each(|(_key, (func, objects))| {
            func.trace();
            for (obj, held_value) in objects {
                if obj.inner.flag == GcFlag::Garbage {
                    continue;
                }
                obj.inner.to_mut().flag = GcFlag::Finalize;
                held_value.trace();
            }
        });
    }

    pub unsafe fn garbage_collect(&mut self, runtime: &Runtime) {
        for (_key, (func, objects)) in &mut self.values {
            let mut stack = Vec::with_capacity(128);
            for (obj, held_value) in objects.iter_mut() {
                if obj.inner.flag == GcFlag::Finalize {
                    stack.resize(1, *held_value);
                    (*func).call(runtime, runtime.global.into(), stack.as_mut_ptr(), 1);

                    obj.inner.to_mut().flag = GcFlag::NotUsed;
                    // todo: clean up registry
                }
            }
        }
    }
}
