use crate::runtime::Profiler;
use crate::runtime::Runtime;
use crate::types::JValue;

mod inlining;

// cranelift somehow cannot compile properly, more test is needed
//mod function_builder;
//pub use function_builder::JSFunctionBuilder;

pub mod llvm;
mod wasm;

pub use crate::bytecodes::optimize::optimize;
use inkwell::execution_engine::JitFunction;

use self::llvm::JSJITFunction;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ExitCode {
    // value0 is used
    Return = 0,
    // value0 is used
    Error = 1,
    // value0 is th future, value[1..2] are registers
    Await = 2,
    // value0 is the yielded value, value[1..2] are registers
    Yield = 3,
    Done = 4,
}

impl ExitCode {
    pub fn is_return(self) -> bool {
        self == Self::Return
    }

    pub fn is_error(self) -> bool {
        self == Self::Error
    }

    pub fn is_yield(self) -> bool {
        self == Self::Error
    }

    pub fn is_done(self) -> bool {
        self == Self::Done
    }
}

pub struct Function {
    pub(crate) profiler: Profiler,
    pub(crate) function: JitFunction<'static, JSJITFunction>,
}

impl Function {
    pub fn call(
        &self,
        runtime: &Runtime,
        this: JValue,
        argc: usize,
        stack: *mut JValue,
        op_stack: *mut JValue,
        capture_stack: *mut JValue,
        async_counter: *mut u32,
        yield_value: JValue,
        r0: JValue,
        r1: JValue,
        r2: JValue,
    ) -> (JValue, JValue, JValue, JValue, ExitCode) {
        unsafe {
            let re = self.function.call(
                this,
                runtime,
                stack,
                argc,
                stack.add(argc),
                op_stack,
                capture_stack,
                async_counter,
                yield_value,
                r0,
                r1,
                r2,
                self.profiler.current
            );

            self.profiler.finish();

            (
                JValue(re[0]),
                JValue(re[1]),
                JValue(re[2]),
                JValue(re[3]),
                std::mem::transmute(re[4] as u8),
            )
        }
    }
}
