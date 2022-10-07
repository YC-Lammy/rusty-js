use std::alloc::Layout;

use crate::runtime::Profiler;
use crate::runtime::Runtime;
use crate::types::JValue;

pub mod bytecode_builder;
mod function_builder;
mod function_builder_context;

pub struct Function {
    profiler: Profiler,
    /// fn(...) -> (result, is_error)
    func: fn(
        this: JValue,
        ctx: &Runtime,
        stack: *mut JValue,
        argc: u32,
        capture_stack: *mut JValue,
    ) -> (JValue, bool),
    code_size: usize,
}

impl Function {
    pub fn call(
        &self,
        runtime: &Runtime,
        this: JValue,
        argc: u32,
        stack: *mut JValue,
        capture_stack: *mut JValue,
    ) -> (JValue, bool) {
        (self.func)(this, runtime, stack, argc, capture_stack)
    }
}

impl Drop for Function {
    fn drop(&mut self) {
        unsafe {
            let ptr = std::mem::transmute::<_, *mut u8>(self.func);
            std::alloc::dealloc(ptr, Layout::array::<u8>(self.code_size).unwrap());
        }
    }
}
