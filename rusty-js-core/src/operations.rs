use std::borrow::Cow;

use crate::bultins::object::JObject;
use crate::bultins::promise::Promise;
use crate::fast_iter::FastIterator;
use crate::runtime::{AsyncResult, FuncID, Runtime, TemplateID};
use crate::types::JValue;

pub fn async_wait(value: JValue) -> (JValue, bool) {
    if let Some(p) = value.as_promise() {
        match p {
            Promise::Fulfilled(v) => (*v, false),
            Promise::Rejected(v) => (*v, true),
            Promise::Pending { id } => {
                let runtime = Runtime::current();

                loop {
                    let re = runtime.to_mut().poll_async(*id, JValue::UNDEFINED);
                    match re {
                        AsyncResult::Err(e) => return (e, true),
                        AsyncResult::Return(r) => return (r, false),
                        // ignore yield value
                        AsyncResult::Yield(_) => {
                            // suspend execution
                            runtime.async_executor.suspend(JValue::UNDEFINED);
                        }
                    }
                }
            }
        }
    } else {
        return (value, false);
    }
}

pub fn Yield(value: JValue) -> JValue {
    let runtime = Runtime::current();
    runtime.generator_executor.suspend(value)
}

pub unsafe fn spread(value: JValue, this: JValue, stack: *mut JValue) -> (*mut JValue, u64, bool) {
    let iter = FastIterator::new(value, crate::bytecodes::LoopHint::For);

    let mut values = Vec::new();
    loop {
        let (done, error, value) = iter.next(this, stack);

        if error{
            return (Box::leak(Box::new(value)), 1, true)
        }

        values.push(value);

        if done {
            FastIterator::drop_(iter);
            break;
        }
    };

    let mut v = Vec::with_capacity(values.len());
    v.extend_from_slice(&values);
    let v = v.leak();
    
    return (v.as_mut_ptr(), v.len() as u64, false)
}

pub unsafe fn create_template(id: u32, args: *mut JValue, argc: u32, tagged: bool) -> JValue {
    let args = std::slice::from_raw_parts_mut(args, argc as usize);
    let runtime = Runtime::current();
    let tpl = runtime.get_template(TemplateID(id));

    if tagged {
        let array = JObject::array();
        for i in &tpl.strings {
            array
                .as_array()
                .unwrap()
                .push((Default::default(), JValue::String(i.as_str().into())))
        }
    }

    let mut exprs = Vec::new();
    for i in args {
        if i.is_string() {
            exprs.push(Cow::Borrowed(i.value.string.as_ref()));
        } else {
            exprs.push(Cow::Owned(i.to_string()))
        };
    }
    tpl.create(&exprs)
}

pub unsafe fn create_function(id: u32, capture_stack: *mut JValue) -> JValue {
    let runtime = Runtime::current();
    let func = runtime.get_function(FuncID(id)).unwrap();
    let ins = func.create_instance_with_capture(None, capture_stack);
    JObject::with_function(ins).into()
}
