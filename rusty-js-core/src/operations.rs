
use crate::bultins::promise::Promise;
use crate::runtime::{
    Runtime,
    AsyncResult
};
use crate::types::JValue;

pub fn Await(value:JValue) -> (JValue, bool){
    if let Some(p) = value.as_promise(){
        match p{
            Promise::Fulfilled(v) => (*v, false),
            Promise::Rejected(v) => (*v, true),
            Promise::Pending { id } => {
                let runtime = Runtime::current();

                loop{
                    let re = runtime.to_mut().poll_async(*id, JValue::UNDEFINED);
                    match re{
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
    } else{
        return (value, false)
    }
}