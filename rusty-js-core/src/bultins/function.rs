use std::alloc::Layout;
use std::sync::Arc;

use crate::baseline;
use crate::bytecodes::OpCode;
use crate::runtime::Runtime;
use crate::types::JValue;

use super::object::JObject;

#[derive(Clone)]
pub struct JSFunctionInstance {
    capture_stack: CaptureStack,
    this: Option<JValue>,
    func: Arc<JSFunction>,
}

impl JSFunctionInstance {
    pub fn Call(
        &self,
        runtime: &Runtime,
        this: JValue,
        stack: *mut JValue,
        argc: usize,
    ) -> (JValue, bool) {

        let this = if let Some(t) = self.this { t } else { this };

        let mut need_drop: Option<CaptureStack> = None;

        
        let capture_stack = match self.capture_stack {
            CaptureStack::Allocated(a) => a.to_mut().as_data(),
            CaptureStack::NeedAlloc(n) => {
                let inner = CaptureStackInner::alloc(n as usize);
                inner.increment_count();
                let ptr = inner.to_mut().as_data();
                need_drop = Some(CaptureStack::Allocated(inner));
                ptr
            }
            CaptureStack::None => &mut [],
        };
        let re = self.func.Call(runtime, this, stack, argc, capture_stack);
        drop(need_drop);
        return re;
    }
}

pub enum CaptureStack {
    NeedAlloc(u32),
    Allocated(&'static CaptureStackInner),
    None,
}

#[repr(C)]
pub struct CaptureStackInner {
    rc: usize,
    alloc: usize,
}

impl CaptureStackInner {
    fn to_mut(&self) -> &mut Self {
        unsafe { std::mem::transmute_copy(&self) }
    }

    fn alloc(size: usize) -> &'static mut CaptureStackInner {
        let size = std::mem::size_of::<CaptureStackInner>() + size * std::mem::size_of::<JValue>();
        let ptr = unsafe { std::alloc::alloc(Layout::array::<u8>(size).unwrap()) };
        let ptr = unsafe { (ptr as *mut CaptureStackInner).as_mut().unwrap() };
        *ptr = CaptureStackInner { rc: 0, alloc: size };
        return ptr;
    }

    fn as_data(&self) -> &'static mut [JValue] {
        let ptr = unsafe { (self as *const Self).add(1) as *mut JValue };
        let len =
            (self.alloc - std::mem::size_of::<CaptureStackInner>()) / std::mem::size_of::<JValue>();
        unsafe { std::slice::from_raw_parts_mut(ptr, len) }
    }

    fn from_data_ptr(ptr: *mut JValue) -> &'static mut Self {
        unsafe { (ptr as *mut CaptureStackInner).sub(1).as_mut().unwrap() }
    }

    fn increment_count(&mut self) {
        self.rc += 1;
    }

    fn decrement_count(&mut self) {
        self.rc -= 1;
    }
}

impl Clone for CaptureStack {
    fn clone(&self) -> Self {
        match self {
            Self::NeedAlloc(size) => Self::NeedAlloc(*size),
            Self::Allocated(a) => {
                let c = unsafe { std::mem::transmute_copy::<_, &mut CaptureStackInner>(a) };
                c.increment_count();
                Self::Allocated(c)
            }
            Self::None => Self::None,
        }
    }
}

impl Drop for CaptureStack {
    fn drop(&mut self) {
        match self {
            Self::Allocated(a) => {
                a.to_mut().rc -= 1;
                if a.rc == 0 {
                    unsafe {
                        std::alloc::dealloc(
                            a.to_mut() as *mut CaptureStackInner as *mut u8,
                            Layout::array::<u8>(a.alloc).unwrap(),
                        )
                    };
                }
            }
            Self::NeedAlloc(_) => {}
            Self::None => {}
        }
    }
}

#[derive(Clone, Copy)]
pub struct JSFuncContext {
    pub(crate) stack: *mut JValue,
}

#[repr(u8)]
pub enum JSFunction {
    ByteCodes {
        is_async: bool,
        is_generator: bool,
        var_count: u16,

        call_count: u16,
        capture_stack_size: Option<u16>,
        bytecodes: Vec<OpCode>,
    },
    Baseline {
        is_async: bool,
        is_generator: bool,
        var_count: u16,

        call_count: u16,
        capture_stack_size: Option<u16>,
        func: baseline::Function,
        bytecodes: Vec<OpCode>,
    },

    Native(Arc<dyn Fn(JSFuncContext, JValue, &[JValue]) -> Result<JValue, JValue>>),
}

impl JSFunction {
    pub fn is_native(&self) -> bool {
        match self {
            Self::Native(_) => true,
            _ => false,
        }
    }

    fn capture_stack_size(&self) -> Option<u16> {
        match self {
            Self::Baseline {
                capture_stack_size, ..
            } => *capture_stack_size,
            Self::Native(_) => None,
            Self::ByteCodes {
                capture_stack_size, ..
            } => *capture_stack_size,
        }
    }

    pub fn create_instance_with_capture(
        self: Arc<Self>,
        this: Option<JValue>,
        capture_stack: *mut JValue,
    ) -> JSFunctionInstance {
        let stack = CaptureStackInner::from_data_ptr(capture_stack);
        stack.increment_count();
        JSFunctionInstance {
            capture_stack: CaptureStack::Allocated(stack),
            this,
            func: self,
        }
    }

    pub fn create_instance(self: Arc<Self>, this: Option<JValue>) -> JSFunctionInstance {
        if self.is_native() {
            return JSFunctionInstance {
                capture_stack: CaptureStack::None,
                this,
                func: self,
            };
        } else {
            if let Some(n) = self.capture_stack_size() {
                return JSFunctionInstance {
                    capture_stack: CaptureStack::NeedAlloc(n as u32),
                    this,
                    func: self,
                };
            } else {
                panic!("create insrance on function that requires capture.")
            }
        }
    }

    fn to_mut(&self) -> &mut Self {
        unsafe { std::mem::transmute_copy(&self) }
    }

    #[allow(non_snake_case)]
    pub fn Call(
        &self,
        runtime: &Runtime,
        this: JValue,
        stack: *mut JValue,
        argc: usize,
        capture_stack: &mut [JValue],
    ) -> (JValue, bool) {
        match self.to_mut() {
            Self::ByteCodes {
                is_async,
                is_generator,
                var_count,
                call_count,
                capture_stack_size: _,
                bytecodes,
            } => {
                if stack.is_null() {
                    panic!()
                }

                *call_count += 1;

                if !*is_async && !*is_generator {
                    let size = unsafe { stack.add(argc).offset_from(runtime.stack.as_ptr()) };
                    let size = runtime.stack.len() - size as usize;
                    let stack_ =
                        unsafe { std::slice::from_raw_parts_mut(stack.add(argc), size as usize) };
                    let args = unsafe { std::slice::from_raw_parts(stack, argc) };

                    let mut intpr =
                        crate::interpreter::Interpreter::function(runtime, stack_, capture_stack);

                    match intpr.run(this, args, &bytecodes) {
                        Ok(v) => (v, false),
                        Err(e) => (e, true),
                    }
                } else if *is_async && !*is_generator {
                    let codes = bytecodes.clone();
                    let var_count = *var_count;

                    // make sure the CaptureStack live long enough
                    let cap = CaptureStackInner::from_data_ptr(capture_stack.as_mut_ptr());
                    cap.increment_count();

                    let p = runtime.to_mut().call_async(move || {
                        let args = unsafe { std::slice::from_raw_parts(stack, argc) };
                        let runtime = Runtime::current();
                        let stack = runtime.to_mut().get_async_stack(var_count as usize);

                        let mut intpr = crate::interpreter::Interpreter::function(
                            &runtime,
                            stack,
                            cap.as_data(),
                        );

                        let re = intpr.run(this, args, &codes);

                        // release the capture stack
                        cap.to_mut().decrement_count();

                        return re;
                    });

                    (JObject::with_promise(p).into(), false)
                } else {
                    todo!()
                }
            }

            Self::Baseline {
                is_async,
                is_generator,
                var_count,
                call_count,
                capture_stack_size: _,
                func,
                bytecodes: _,
            } => {
                if stack.is_null() {
                    panic!()
                }

                *call_count += 1;
                func.call(
                    runtime,
                    this,
                    argc as u32,
                    stack,
                    capture_stack.as_mut_ptr(),
                )
            }

            Self::Native(n) => {
                // the stack pointer can be null

                let args = unsafe { std::slice::from_raw_parts(stack as *const JValue, argc) };

                let ctx = JSFuncContext {
                    stack: unsafe { stack.add(argc) },
                };

                let re = (n)(ctx, this, args);
                match re {
                    Ok(v) => (v, false),
                    Err(v) => (v, true),
                }
            }
        }
    }
}
