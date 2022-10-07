use std::sync::Arc;

use crate::{
    bultins::{
        object::{JObject, JObjectValue},
        prop::PropFlag,
    },
    bytecodes::LoopHint,
    error::Error,
    runtime::Runtime,
    types::JValue,
};

pub enum FastIterator {
    Array {
        array: Vec<(PropFlag, JValue)>,
        count: u32,
    },
    Object {
        obj: JObject,
    },
    // for (... in obj)
    ForIn {
        keys: &'static [u32],
        count: u32,
    },
    Empty,
}

#[test]
fn test_fast_iter_size() {
    assert!(std::mem::size_of::<FastIterator>() == 16)
}

#[allow(unused)]
impl FastIterator {
    pub unsafe fn new(iter: JValue, hint: LoopHint) -> &'static mut FastIterator {
        Box::leak(Box::new(if hint == LoopHint::ForIn {
            if iter.is_object() {
                let keys = iter.value.object.keys();
                FastIterator::ForIn {
                    keys: keys,
                    count: 0,
                }
            } else {
                FastIterator::Empty
            }
        } else {
            if iter.is_object() {
                if iter.value.object.is_array() {
                    FastIterator::Array {
                        array: iter
                            .value
                            .object
                            .inner
                            .wrapped_value
                            .array()
                            .unwrap()
                            .clone(),
                        count: 0,
                    }
                } else {
                    FastIterator::Object {
                        obj: iter.value.object,
                    }
                }
            } else {
                FastIterator::Empty
            }
        }))
    }
    /// return (done, error, value)
    pub fn next(&mut self, this: JValue, stack: *mut JValue) -> (bool, bool, JValue) {
        match self {
            Self::Empty => (true, false, JValue::UNDEFINED),
            Self::Array { array, count } => {
                if array.len() > *count as usize + 1 {
                    *count += 1;
                    (false, false, array.get(*count as usize).unwrap().1)
                } else if array.len() == *count as usize {
                    let v = (true, false, array.get(*count as usize).unwrap().1);
                    *self = Self::Empty;
                    return v;
                } else {
                    (true, false, JValue::UNDEFINED)
                }
            }
            Self::Object { obj } => {
                let next = match obj.get_property("next", stack) {
                    Some(v) => v,
                    None => {
                        return (
                            false,
                            true,
                            JValue::Error(Error::InvalideIterator { msg: "" }),
                        )
                    }
                };

                match next.call(
                    crate::bultins::function::JSFuncContext { stack: stack },
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

        let object = JObject::with_value(JObjectValue::Array(values));
        (JValue::Object(object), false)
    }

    pub fn drop_(&'static mut self) {
        unsafe { Box::from_raw(self) };
    }
}
