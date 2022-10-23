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
        argc: usize,
        capture_stack: *mut JValue,
    ) -> (JValue, bool),
    m:memmap2::Mmap
}

impl Function {
    pub fn call(
        &self,
        runtime: &Runtime,
        this: JValue,
        argc: usize,
        stack: *mut JValue,
        capture_stack: *mut JValue,
    ) -> (JValue, bool) {
        (self.func)(this, runtime, stack, argc, capture_stack)
    }
}