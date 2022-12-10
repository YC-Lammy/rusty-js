use std::sync::Arc;

use crate::{
    bultins::{object::JObject, flag::PropFlag},
    bytecodes::LoopHint,
    runtime::Runtime,
    types::JValue,
    utils::string_interner::SYMBOLS, JSContext,
};

pub enum FastIterator {
    Array {
        array: Arc<Vec<(PropFlag, JValue)>>,
        count: u32,
    },
    Object {
        obj: JObject,
        iter: Option<JObject>,
    },
    // for (... in obj)
    ForIn {
        keys: &'static [u32],
        count: u32,
    },
    Empty,
}

#[allow(unused)]
impl FastIterator {
    #[inline]
    pub unsafe fn new(iter: JValue, hint: LoopHint) -> Result<Self, JValue> {
        Box::leak(Box::new(if hint == LoopHint::ForIn {
            if let Some(obj) = iter.is_object() {
                let keys = obj.keys();

                Ok(FastIterator::ForIn {
                    keys: keys,
                    count: 0,
                })
            } else {
                Ok(FastIterator::Empty)
            }
        } else {
            if let Some(obj) = iter.as_object() {
                if let Some(ar) = obj.as_arc_array() {
                    Ok(
                        FastIterator::Array {
                        array: ar.clone(),
                        count: 0,
                    })
                } else {
                    let iter = iter.get_property_static(SYMBOLS["iterator"], std::ptr::null_mut())
                        .unwrap();

                    FastIterator::Object {
                        obj: iter.value.object,
                        iter: None,
                    }
                }
            } else {
                FastIterator::Empty
            }
        }))
    }
    /// return (done, error, value)
    #[inline]
    pub fn next(&mut self, this: JValue, stack: *mut JValue) -> (bool, bool, JValue) {
        match self {
            Self::Empty => (true, false, JValue::UNDEFINED),
            Self::Array { array, count } => {
                if array.len() > *count as usize + 1 {
                    *count += 1;
                    (false, false, array.get(*count as usize).unwrap().1)
                } else {
                    (true, false, JValue::UNDEFINED)
                }
            }
            Self::Object { obj, iter } => {
                let next = match iter.unwrap().get_property("next", JSContext{
                    stack:stack,
                    runtime:&Runtime::current(),
                }) {
                    Ok(v) => v,
                    Err(e) => return (false, true, e),
                };

                match next.call(
                    &crate::bultins::function::JSContext {
                        stack: stack,
                        runtime: &Runtime::current(),
                    },
                    this,
                    &[],
                ) {
                    Ok(result) => {
                        let done = match result.get_property_str("done") {
                            Ok(v) => v,
                            Err(v) => return (true, true, v),
                        };
                        let v = match result.get_property_str("value") {
                            Ok(v) => v,
                            Err(v) => return (true, true, v),
                        };

                        (done.to_bool(), false, v)
                    }
                    Err(e) => (true, true, e),
                }
            }
            Self::ForIn { keys, count } => {
                let runtime = Runtime::current();
                if keys.len() > *count as usize + 1 {
                    *count += 1;

                    let key = keys[*count as usize];
                    (
                        false,
                        false,
                        JValue::String(runtime.get_field_name(key).into()),
                    )
                } else if keys.len() == *count as usize {
                    let key = keys[*count as usize];
                    let v = (
                        true,
                        false,
                        JValue::String(runtime.get_field_name(key).into()),
                    );
                    *self = Self::Empty;
                    return v;
                } else {
                    (true, false, JValue::UNDEFINED)
                }
            }
        }
    }

    pub fn collect(&mut self, this: JValue, stack: *mut JValue) -> (JValue, bool) {
        let mut values = vec![];
        loop {
            let (done, error, value) = self.next(this, stack);
            if error {
                return (value, true);
            };
            values.push((PropFlag::THREE, value));
            if done {
                break;
            }
        }

        let object = JObject::with_array(values);
        (JValue::create_object(object), false)
    }

    pub fn drop_(&'static mut self) {
        unsafe { Box::from_raw(self) };
    }
}
