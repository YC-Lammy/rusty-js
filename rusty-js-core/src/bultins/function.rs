use std::alloc::Layout;
use std::sync::Arc;

use crate::baseline;
use crate::bytecodes::OpCode;
use crate::runtime::Runtime;
use crate::types::JValue;

use super::object::{JObject, JObjectValue};
use super::prop::PropFlag;

#[derive(Clone)]
pub struct JSFunctionInstance {
    capture_stack: CaptureStack,
    this: Option<JValue>,
    pub(crate) func: Arc<JSFunction>,
}

impl JSFunctionInstance {
    pub fn call(
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

    pub unsafe fn trace(&self){
        if let Some(v) = &self.this{
            v.trace();
        }

        match &self.capture_stack{
            CaptureStack::Allocated(a) => {
                for i in a.as_data(){
                    i.trace();
                }
            },
            _ => {}
        };
    }

    #[inline]
    pub fn create_object(self) -> JObject{
        let rt = Runtime::current();
        let mut obj = JObject::new();
        let mut proto = JObject::new();
        
        proto.insert_property("constructor", obj.into(), PropFlag::CONFIGURABLE|PropFlag::WRITABLE);

        obj.insert_property("length", JValue::Number(self.func.args_len() as f64), PropFlag::CONFIGURABLE);
        obj.insert_property("name", JValue::String("".into()), PropFlag::CONFIGURABLE);
        obj.insert_property("prototype", proto.into(), PropFlag::NONE);
        obj.insert_property("__proto__", rt.prototypes.function.into(), Default::default());

        obj.inner.to_mut().wrapped_value = JObjectValue::Function(self);
        return obj
    }
}

// todo: use garbage collect CaptureStack
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

        for i in ptr.as_data(){
            *i = JValue::UNDEFINED;
        }
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
        args_len:u16,

        call_count: u16,
        capture_stack_size: Option<u16>,
        bytecodes: Vec<OpCode>,
    },
    Baseline {
        is_async: bool,
        is_generator: bool,
        var_count: u16,
        args_len:u16,

        call_count: u16,
        capture_stack_size: Option<u16>,
        func: Arc<baseline::Function>,
        bytecodes: Vec<OpCode>,
    },

    Native(Arc<dyn Fn(&JSFuncContext, JValue, &[JValue]) -> Result<JValue, JValue>>),
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

    pub fn args_len(&self) -> usize{
        match self {
            Self::Baseline {args_len, ..} => {
                *args_len as usize
            },
            Self::ByteCodes {args_len, ..} => {
                *args_len as usize
            },
            Self::Native(_) => {
                0
            }
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
                args_len:_,
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

                        let oldstack = stack;
                        let runtime = Runtime::current();
                        let stack = runtime.to_mut().get_async_stack(var_count as usize);

                        unsafe{std::ptr::copy(oldstack, stack.as_mut_ptr(), argc)};

                        let stack_ptr = stack.as_mut_ptr();
                        let args = unsafe{std::slice::from_raw_parts(stack_ptr, argc)};

                        let mut intpr = crate::interpreter::Interpreter::function(
                            &runtime,
                            &mut stack[argc..],
                            cap.as_data(),
                        );

                        runtime.user_own_value(this);

                        let re = intpr.run(this, args, &codes);

                        runtime.user_drop_value(this);

                        // release the capture stack
                        cap.to_mut().decrement_count();

                        return re;
                    });

                    (JObject::with_promise(p).into(), false)

                } else {
                    todo!("generator function")
                }
            }

            Self::Baseline {
                is_async,
                is_generator,
                var_count,
                args_len:_,
                call_count,
                capture_stack_size: _,
                func,
                bytecodes: _,
            } => {
                if stack.is_null() {
                    panic!()
                }
                
                *call_count += 1;

                if !*is_async && !*is_generator{
                    
                    func.call(
                        runtime,
                        this,
                        argc as u32,
                        stack,
                        capture_stack.as_mut_ptr(),
                    )
                } else if *is_async && !*is_generator{

                    // make sure the CaptureStack live long enough
                    let cap = CaptureStackInner::from_data_ptr(capture_stack.as_mut_ptr());
                    cap.increment_count();

                    let f = func.clone();
                    let var_count = *var_count as usize;

                    let p = runtime.to_mut().call_async(move ||{

                        let oldstack = stack;
                        let runtime = Runtime::current();
                        let stack = runtime.to_mut().get_async_stack(var_count);

                        unsafe{std::ptr::copy(oldstack, stack.as_mut_ptr(), argc)};

                        runtime.user_own_value(this);

                        let re = f.call(&runtime, this, argc as u32, stack.as_mut_ptr(), cap.as_data().as_mut_ptr());

                        runtime.user_drop_value(this);

                        // drop the capture stack
                        cap.to_mut().decrement_count();

                        if re.1{
                            return Err(re.0)
                        } else{
                            return Ok(re.0)
                        }
                    });

                    return (JObject::with_promise(p).into(), false)

                } else{
                    todo!("generator function")
                }
                
            }

            Self::Native(n) => {
                // the stack pointer can be null

                let args = unsafe { std::slice::from_raw_parts(stack as *const JValue, argc) };

                let ctx = JSFuncContext {
                    stack: unsafe { stack.add(argc) },
                };

                let re = (n)(&ctx, this, args);
                match re {
                    Ok(v) => (v, false),
                    Err(v) => (v, true),
                }
            }
        }
    }
}
