use std::sync::Arc;

use corosensei::{Coroutine, CoroutineResult, Yielder};
use corosensei::stack::DefaultStack;

use crate::types::JValue;

pub enum GeneratorResult{
    Yield(JValue),
    Return(JValue),
    Error(JValue),
    Finished
}


pub struct Generator{
    is_finished:bool,
    func:Arc<dyn Fn(&Yielder<JValue, JValue>, JValue) -> Result<JValue, JValue>>,
    coroutine:Coroutine<JValue, JValue, Result<JValue, JValue>, DefaultStack>,
}

impl Generator{
    pub fn next(&mut self, input:JValue) -> GeneratorResult{

        if self.is_finished {
            return GeneratorResult::Finished
        }
        let re = self.coroutine.resume(input);
        match re{
            CoroutineResult::Return(r) => {
                self.is_finished = true;

                match r{
                    Ok(v) => GeneratorResult::Return(v),
                    Err(e) => GeneratorResult::Error(e)
                }
            },
            CoroutineResult::Yield(y) => {
                GeneratorResult::Yield(y)
            }
        }
    }
}

impl Clone for Generator{
    fn clone(&self) -> Self {
        let func = self.func.clone();
        Self { 
            is_finished: false, 
            func: self.func.clone(), 
            coroutine: Coroutine::new(move |y, i|{
                (func)(y, i)
            })
        }
    }
}