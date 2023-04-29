use std::alloc::Layout;
use std::sync::Arc;

use crate::baseline;
use crate::bytecodes::OpCode;
use crate::interpreter::{clousure, Interpreter};
use crate::runtime::{Runtime, DEFAULT_STACK_SIZE};
use crate::value::JValue;
use crate::utils::string_interner::NAMES;

use super::object_property::PropFlag;
use super::object::{JObject, JObjectValue};

#[derive(Clone)]
pub struct JSFunctionInstance {
    capture_stack: CaptureStack,
    this: Option<JValue>,
    pub(crate) func: Arc<JSFunction>,
}

impl JSFunctionInstance {
    pub fn call(
        self: Arc<Self>,
        runtime: &Runtime,
        this: JValue,
        stack: *mut JValue,
        argc: usize,
    ) -> (JValue, bool) {
        let this = if let Some(t) = self.this { t } else { this };

        let re = JSFunction::call(
            self.func.to_mut(),
            runtime,
            this,
            stack,
            argc,
            self.capture_stack.clone(),
        );

        match re {
            Ok(v) => (v, false),
            Err(e) => (e, true),
        }
    }

    pub fn capture_stack(&self) -> Option<Arc<Box<[JValue]>>> {
        match &self.capture_stack {
            CaptureStack::Allocated(a) => Some(a.clone()),
            CaptureStack::NeedAlloc(n) => {
                let data = unsafe {
                    std::alloc::alloc_zeroed(Layout::array::<JValue>(*n as usize).unwrap())
                } as *mut JValue;
                let data = unsafe { std::slice::from_raw_parts_mut(data, *n as usize) };
                let data = unsafe { Box::from_raw(data) };
                Some(Arc::new(data))
            }
            CaptureStack::None => None,
        }
    }

    pub unsafe fn trace(&self) {
        if let Some(v) = &self.this {
            v.trace();
        }
    }

    #[inline]
    pub fn create_object(self) -> JObject {
        let rt = Runtime::current();
        let obj = JObject::new();
        let proto = JObject::new();

        proto.insert_property(NAMES["constructor"], obj.into(), PropFlag::BUILTIN);

        obj.insert_property(
            NAMES["length"],
            JValue::create_number(self.func.args_len as f64),
            PropFlag::CONFIGURABLE,
        );
        obj.insert_property(
            NAMES["name"],
            JValue::create_static_string(""),
            PropFlag::CONFIGURABLE,
        );
        obj.insert_property(NAMES["prototype"], proto.into(), PropFlag::NONE);
        obj.insert_property(
            NAMES["__proto__"],
            rt.prototypes.function.into(),
            Default::default(),
        );

        obj.inner.to_mut().wrapped_value = JObjectValue::Function(Arc::new(self));
        return obj;
    }
}

// todo: use garbage collect CaptureStack
#[derive(Clone)]
pub enum CaptureStack {
    NeedAlloc(u32),
    Allocated(Arc<Box<[JValue]>>),
    None,
}

impl CaptureStack {
    pub fn data(&self) -> Option<Arc<Box<[JValue]>>> {
        match self {
            CaptureStack::Allocated(a) => Some(a.clone()),
            CaptureStack::NeedAlloc(n) => {
                let data = unsafe {
                    std::alloc::alloc_zeroed(Layout::array::<JValue>(*n as usize).unwrap())
                } as *mut JValue;
                let data = unsafe { std::slice::from_raw_parts_mut(data, *n as usize) };
                let data = unsafe { Box::from_raw(data) };
                Some(Arc::new(data))
            }
            CaptureStack::None => None,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct JSContext<'a> {
    pub stack: *mut JValue,
    pub runtime: &'a Runtime,
}

#[repr(C)]
pub struct JSFunction {
    pub is_async: bool,
    pub is_generator: bool,
    pub var_count: u16,
    pub args_len: u16,
    pub largest_stack_offset: u32,

    pub call_count: u64,
    pub capture_stack_size: Option<u16>,

    pub bytecodes: Arc<Vec<OpCode>>,

    pub baseline_clousure: Option<clousure::Clousure>,
    pub baseline_jit: Option<Arc<baseline::Function>>,
}

impl JSFunction {
    pub fn create_instance(self: Arc<Self>, this: Option<JValue>) -> JSFunctionInstance {
        if let Some(n) = self.capture_stack_size {
            let cap = if n == 0 {
                CaptureStack::None
            } else {
                CaptureStack::NeedAlloc(n as u32)
            };
            return JSFunctionInstance {
                capture_stack: cap,
                this,
                func: self,
            };
        } else {
            return JSFunctionInstance {
                capture_stack: CaptureStack::None,
                this,
                func: self,
            };
        }
    }

    pub fn create_instance_with_capture(
        self: Arc<Self>,
        this: Option<JValue>,
        capture_stack: CaptureStack,
    ) -> JSFunctionInstance {
        return JSFunctionInstance {
            capture_stack: capture_stack,
            this,
            func: self,
        };
    }

    fn to_mut(&self) -> &mut Self {
        unsafe { std::mem::transmute_copy(&self) }
    }

    pub fn call(
        &mut self,
        runtime: &Runtime,
        this: JValue,
        stack: *mut JValue,
        argc: usize,
        capture_stack: CaptureStack,
    ) -> Result<JValue, JValue> {
        self.call_count += 1;

        if self.call_count == 10 {
            self.compile(runtime);
        } else if self.call_count == 100 {
        } else if self.call_count == 10000 {
        }

        if !self.is_async && !self.is_generator {

            if let Some(f) = &self.baseline_jit {
                /*
                if argc < self.args_len as usize {
                    let rargs = unsafe { stack.add(argc) };
                    let s = unsafe {
                        std::slice::from_raw_parts_mut(rargs, self.args_len as usize - argc)
                    };
                    s.fill(JValue::UNDEFINED);
                    argc = self.args_len as usize;
                };*/

                let cd = capture_stack.data();
                let cap = cd
                    .and_then(|v| Some(v.as_ref().as_ref().as_ptr() as *mut JValue))
                    .unwrap_or(std::ptr::null_mut());

                let mut counter = 0;

                let (v0, _v1, _v2, _v3, exit) = f.call(
                    runtime,
                    this,
                    argc,
                    stack,
                    unsafe { stack.add(self.largest_stack_offset as usize + 1) },
                    cap,
                    &mut counter,
                    JValue::UNDEFINED,
                    JValue::UNDEFINED,
                    JValue::UNDEFINED,
                    JValue::UNDEFINED,
                );

                if exit.is_error() {
                    return Err(v0);
                } else {
                    return Ok(v0);
                };

            } else if let Some(c) = &mut self.baseline_clousure {
                let stack = unsafe { std::slice::from_raw_parts_mut(stack, DEFAULT_STACK_SIZE) };
                let op_stack = unsafe {
                    std::slice::from_raw_parts_mut(
                        stack
                            .as_mut_ptr()
                            .add(self.largest_stack_offset as usize + argc),
                        DEFAULT_STACK_SIZE,
                    )
                };
                let args = unsafe { std::slice::from_raw_parts_mut(stack.as_mut_ptr(), argc) };

                let cd = capture_stack.data();
                let cap = cd.as_ref().and_then(|v| Some(v.as_ref().as_ref()));
                let cap = unsafe { std::mem::transmute_copy(&cap) };

                c.run(
                    runtime,
                    &mut stack[argc..],
                    op_stack,
                    Some(capture_stack),
                    cap,
                    this,
                    args,
                )
            } else {
                let stack = unsafe { std::slice::from_raw_parts_mut(stack, DEFAULT_STACK_SIZE) };
                let cd = capture_stack.data();
                let cap = cd.as_ref().and_then(|v| Some(v.as_ref().as_ref()));
                let cap = unsafe { std::mem::transmute_copy(&cap) };

                let args = unsafe { std::slice::from_raw_parts_mut(stack.as_mut_ptr(), argc) };

                let mut intpr = Interpreter::function(
                    runtime,
                    &mut stack[argc..],
                    argc + self.largest_stack_offset as usize,
                    capture_stack,
                    cap,
                );

                intpr.run(this, args, &self.bytecodes)
            }
        } else {
            todo!()
        }
    }

    fn compile(&mut self, rt: &Runtime) {
        if self.baseline_jit.is_none() {
            let ptr = &mut self.baseline_jit as *mut _ as usize;
            let runtime = Runtime::current();
            let bytecodes = self.bytecodes.clone();

            rt.worker_task_sender.send(Box::new(move ||{
                // todo: identify the static borrow and use arc instead
                let ctx_a= runtime.baseline_context.clone();
                let module_a = runtime.baseline_module.clone();
                let engine_a = runtime.baseline_engine.as_ref().unwrap().clone();

                let ctx = unsafe{std::mem::transmute_copy(&ctx_a.as_ref())};
                let module = unsafe{std::mem::transmute_copy(&module_a.as_ref())};
                let engine = unsafe{std::mem::transmute_copy(&engine_a.as_ref())};

                let mut codegen = baseline::llvm::CodeGen::new(
                    ctx,
                    module,
                    engine,
                );
                let func = codegen.translate_codes(&bytecodes);

                unsafe{
                    (ptr as *mut Option<Arc<baseline::Function>>).write(Some(Arc::new(func)));
                }
            })).expect("failed to send task");
        }

    }
}
