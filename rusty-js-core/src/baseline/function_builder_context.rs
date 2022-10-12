use std::collections::HashMap;
use std::sync::Arc;

use swc_ecmascript::ast::Id;

pub use crate::bytecodes::DeclareKind;
use crate::bytecodes::{OpCode, Register};
use crate::runtime::Runtime;

pub type StackOffset = u16;

#[repr(u8)]
pub enum Variable {
    Let(StackOffset),
    Var(StackOffset),
    Const(StackOffset),
    /// Captured variable will be stored into runtime context
    /// and will be read/write dynamically
    Captured(StackOffset),
}

#[derive(Clone)]
pub struct FunctionBuilderContext {
    pub(crate) inner: Arc<FunctionBuilderContextInner>,
}

pub struct FunctionBuilderContextInner {
    parent: Option<Arc<FunctionBuilderContextInner>>,
    is_function_context: bool,
    is_global_context: bool,

    pub is_try: bool,
    variables: HashMap<Id, Variable>,
    stack_offset: StackOffset,
    capture_offset: Option<u16>,

    max_stack_offset: u16,
    pub(crate) need_done: Vec<OpCode>,
}

impl FunctionBuilderContext {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(FunctionBuilderContextInner {
                parent: None,
                is_function_context: true,
                is_global_context: true,
                is_try: false,
                variables: Default::default(),
                stack_offset: 0,
                capture_offset: None,
                max_stack_offset: 0,
                need_done: vec![],
            }),
        }
    }

    pub fn need_done(&self) -> Vec<OpCode> {
        let mut v = vec![];
        let s = self.inner.to_mut();
        std::mem::swap(&mut s.need_done, &mut v);
        
        if self.inner.is_function_context || self.inner.is_global_context{
            return v;
        }

        let mut pa = self.inner.parent.clone();
        let mut should_break = false;
        loop{
            if should_break{
                break;
            }
            if let Some(p) = &pa{
                if p.is_global_context{
                    break;
                } 
                
                if p.is_function_context{
                    should_break = true;
                }

                let mut f = Vec::new();
                std::mem::swap(&mut p.to_mut().need_done, &mut f);
                v.extend(f);
                pa = p.parent.clone();
            } else{
                break
            }
        }
        return v;
    }

    pub fn new_context(&mut self) {
        let ctx = FunctionBuilderContextInner {
            parent: Some(self.inner.clone()),
            variables: Default::default(),

            is_function_context: false,
            is_global_context: self.inner.is_global_context,
            is_try: self.inner.is_try,
            stack_offset: self.inner.stack_offset,
            max_stack_offset: 0,
            capture_offset: None,
            need_done: vec![],
        };
        self.inner = Arc::new(ctx);
    }

    pub fn new_function(&mut self) {
        let ctx = FunctionBuilderContextInner {
            parent: Some(self.inner.clone()),
            variables: Default::default(),

            is_function_context: true,
            is_global_context: false,
            is_try: false,
            stack_offset: 0,
            capture_offset: {
                if self.inner.is_global_context {
                    Some(0)
                } else {
                    None
                }
            },
            max_stack_offset: 0,
            need_done: vec![],
        };
        self.inner = Arc::new(ctx);
    }

    pub fn close_context(&mut self) {
        if let Some(p) = &self.inner.parent {
            if self.inner.need_done.len() != 0 {
                panic!("Bytecode builder context has not yet done every job.")
            }
            self.inner = p.clone();
        } else {
            //panic!("ClousureJit FunctionBuilderContext close on global context.")
        };
    }

    pub fn is_try(&self) -> bool {
        self.inner.is_try
    }

    pub fn set_try(&self) {
        self.inner.to_mut().is_try = true;
    }

    pub fn declare(&mut self, name: Id, from: Register, kind: DeclareKind) -> OpCode {
        self.inner.to_mut().declare(name, from, kind)
    }

    pub fn get(&mut self, name: &Id, write_to: Register) -> OpCode {
        self.inner.to_mut().get(name, write_to)
    }

    pub fn set(&mut self, name: Id, value: Register) -> OpCode {
        self.inner.to_mut().set(name, value)
    }

    pub fn current_stack_offset(&self) -> StackOffset {
        self.inner.stack_offset
    }

    pub fn max_stack_offset(&self) -> StackOffset {
        self.inner.max_stack_offset
    }

    pub fn capture_len(&self) -> Option<u16> {
        self.inner.capture_offset
    }
}

impl AsMut<FunctionBuilderContextInner> for FunctionBuilderContext {
    fn as_mut(&mut self) -> &mut FunctionBuilderContextInner {
        self.inner.to_mut()
    }
}

impl FunctionBuilderContextInner {
    fn to_mut(&self) -> &mut Self {
        unsafe { (self as *const Self as *mut Self).as_mut().unwrap() }
    }

    fn get_next_capture_offset(&self) -> u16 {
        if self.is_global_context {
            panic!("getting capture offset on global context");
        }
        if self.capture_offset.is_some() {
            let off = self.capture_offset.unwrap();
            self.to_mut().capture_offset = Some(off + 1);
            return off;
        }
        let mut parent = self.parent.clone();
        loop {
            if let Some(p) = &parent {
                if p.is_global_context {
                    panic!("getting capture offset on global context");
                }
                if p.capture_offset.is_some() {
                    let off = p.capture_offset.unwrap();
                    p.to_mut().capture_offset = Some(off + 1);
                    return off;
                } else {
                    parent = p.parent.clone();
                }
            } else {
                panic!("cannot capture on non existing context.")
            }
        }
    }

    pub fn get_next_stack_offset(&mut self) -> u16 {
        let off = self.stack_offset;
        self.stack_offset += 1;

        if self.is_global_context {
            return off;
        }

        if self.is_function_context {
            self.max_stack_offset = off.max(self.max_stack_offset);
            return off;
        }

        let mut p = self.parent.clone();
        loop {
            if let Some(pa) = &p {
                if pa.is_function_context {
                    pa.to_mut().max_stack_offset = off.max(pa.max_stack_offset);
                } else {
                    p = pa.parent.clone();
                }
            } else {
                break;
            }
        }
        return off;
    }

    pub fn function_parent(&self) -> Arc<FunctionBuilderContextInner> {
        if self.is_function_context {
            panic!("basline: cannot get parent context of top level context.")
        }
        let mut parent = self.parent.clone();
        loop {
            if let Some(p) = parent {
                if p.is_function_context {
                    return p;
                }
                parent = p.parent.clone();
            } else {
                unreachable!("function builder context must have parent")
            }
        }
    }

    pub fn get(&mut self, name: &Id, write_to: Register) -> OpCode {
        if let Some(v) = self.variables.get(&name) {
            match v {
                Variable::Captured(off) => {
                    return OpCode::ReadCapturedVar {
                        result: write_to,
                        offset: *off,
                    }
                }
                Variable::Const(off) => {
                    if !self.is_global_context {
                        return OpCode::ReadFromStack {
                            result: write_to,
                            stack_offset: *off,
                        };
                    } else {
                        let runtime = Runtime::current();
                        let id = runtime.to_mut().regester_dynamic_var_name(name.0.as_ref());

                        return OpCode::ReadDynamicVar {
                            result: write_to,
                            id: id,
                        };
                    }
                }
                Variable::Let(off) => {
                    if !self.is_global_context {
                        return OpCode::ReadFromStack {
                            result: write_to,
                            stack_offset: *off,
                        };
                    } else {
                        let runtime = Runtime::current();
                        let id = runtime.to_mut().regester_dynamic_var_name(name.0.as_ref());

                        return OpCode::ReadDynamicVar {
                            result: write_to,
                            id: id,
                        };
                    }
                }
                Variable::Var(off) => {
                    if !self.is_global_context {
                        return OpCode::ReadFromStack {
                            result: write_to,
                            stack_offset: *off,
                        };
                    } else {
                        let runtime = Runtime::current();
                        let id = runtime.to_mut().regester_dynamic_var_name(name.0.as_ref());

                        return OpCode::ReadDynamicVar {
                            result: write_to,
                            id: id,
                        };
                    }
                }
            }
        };

        let mut parent = self.parent.clone();
        let mut is_func_ctx = self.is_function_context;
        let mut is_global;
        let mut need_capture = false;

        loop {
            if is_func_ctx {
                need_capture = true
            }

            if let Some(p) = &parent {
                is_func_ctx = p.is_function_context;
                is_global = p.is_global_context;

                if let Some(v) = p.variables.get(&name) {
                    match v {
                        Variable::Captured(off) => {
                            // already captured
                            return OpCode::ReadCapturedVar {
                                result: write_to,
                                offset: *off,
                            };
                        }
                        Variable::Const(off) => {
                            if need_capture && !is_global {
                                let offset = self.get_next_capture_offset();

                                p.to_mut().need_done.push(OpCode::Capture {
                                    stack_offset: *off,
                                    capture_stack_offset: offset,
                                });
                                p.to_mut()
                                    .variables
                                    .insert(name.clone(), Variable::Captured(offset));

                                return OpCode::ReadCapturedVar {
                                    result: write_to,
                                    offset: offset,
                                };
                            } else if is_global {
                                let runtime = Runtime::current();
                                let id =
                                    runtime.to_mut().regester_dynamic_var_name(name.0.as_ref());

                                return OpCode::ReadDynamicVar {
                                    result: write_to,
                                    id: id,
                                };
                            } else {
                                return OpCode::ReadFromStack {
                                    result: write_to,
                                    stack_offset: *off,
                                };
                            }
                        }
                        Variable::Let(l) => {
                            if need_capture && !is_global {
                                let offset = self.get_next_capture_offset();

                                p.to_mut().need_done.push(OpCode::Capture {
                                    stack_offset: *l,
                                    capture_stack_offset: offset,
                                });

                                p.to_mut()
                                    .variables
                                    .insert(name.clone(), Variable::Captured(offset));

                                return OpCode::ReadCapturedVar {
                                    result: write_to,
                                    offset: offset,
                                };
                            } else if is_global {
                                let runtime = Runtime::current();
                                let id =
                                    runtime.to_mut().regester_dynamic_var_name(name.0.as_ref());

                                return OpCode::ReadDynamicVar {
                                    result: write_to,
                                    id: id,
                                };
                            } else {
                                return OpCode::ReadFromStack {
                                    result: write_to,
                                    stack_offset: *l,
                                };
                            }
                        }
                        Variable::Var(v) => {
                            if need_capture && !is_global {
                                let offset = self.get_next_capture_offset();

                                p.to_mut().need_done.push(OpCode::Capture {
                                    stack_offset: *v,
                                    capture_stack_offset: offset,
                                });
                                p.to_mut()
                                    .variables
                                    .insert(name.clone(), Variable::Captured(offset));

                                return OpCode::ReadCapturedVar {
                                    result: write_to,
                                    offset: offset,
                                };
                            } else if is_global {
                                let runtime = Runtime::current();
                                let id =
                                    runtime.to_mut().regester_dynamic_var_name(name.0.as_ref());

                                return OpCode::ReadDynamicVar {
                                    result: write_to,
                                    id: id,
                                };
                            } else {
                                return OpCode::ReadFromStack {
                                    result: write_to,
                                    stack_offset: *v,
                                };
                            }
                        }
                    }
                } else {
                    parent = p.parent.clone();
                }
            } else {
                break;
            }
        }

        // fallback to dynamic get
        let runtime = Runtime::current();
        let id = runtime.regester_dynamic_var_name(&name.0);
        return OpCode::ReadDynamicVar {
            result: write_to,
            id: id,
        };
    }

    fn set(&mut self, name: Id, from: Register) -> OpCode {
        if let Some(v) = self.variables.get(&name) {
            match v {
                Variable::Captured(off) => {
                    return OpCode::WriteCapturedVar {
                        from: from,
                        offset: *off,
                    }
                }
                Variable::Const(off) => {
                    todo!("assign const offset: {}", off)
                }
                Variable::Let(off) => {
                    if self.is_global_context {
                        let runtime = Runtime::current();
                        let id = runtime.regester_dynamic_var_name(name.0.as_ref());

                        return OpCode::WriteDynamicVar { from: from, id: id };
                    }
                    return OpCode::WriteToStack {
                        from: from,
                        stack_offset: *off,
                    };
                }
                Variable::Var(off) => {
                    if self.is_global_context {
                        let runtime = Runtime::current();
                        let id = runtime.regester_dynamic_var_name(name.0.as_ref());

                        return OpCode::WriteDynamicVar { from: from, id: id };
                    }
                    return OpCode::WriteToStack {
                        from: from,
                        stack_offset: *off,
                    };
                }
            }
        };

        let mut parent = self.parent.clone();
        let mut is_func_ctx = self.is_function_context;
        let mut is_global;
        let mut need_capture = false;

        loop {
            if is_func_ctx {
                need_capture = true
            }

            if let Some(p) = &parent {
                is_func_ctx = p.is_function_context;
                is_global = p.is_global_context;

                if let Some(v) = p.variables.get(&name) {
                    match v {
                        Variable::Captured(off) => {
                            // already captured
                            return OpCode::WriteCapturedVar {
                                from: from,
                                offset: *off,
                            };
                        }
                        Variable::Const(off) => {
                            todo!("assign const offset:{}", off)
                        }
                        Variable::Let(l) => {
                            if need_capture && !is_global {
                                let offset = self.get_next_capture_offset();

                                p.to_mut().need_done.push(OpCode::Capture {
                                    stack_offset: *l,
                                    capture_stack_offset: offset,
                                });
                                p.to_mut()
                                    .variables
                                    .insert(name.clone(), Variable::Captured(offset));

                                return OpCode::WriteCapturedVar {
                                    from: from,
                                    offset: offset,
                                };
                            } else if is_global {
                                let runtime = Runtime::current();
                                let id =
                                    runtime.to_mut().regester_dynamic_var_name(name.0.as_ref());

                                return OpCode::WriteDynamicVar { from: from, id: id };
                            } else {
                                return OpCode::WriteToStack {
                                    from: from,
                                    stack_offset: *l,
                                };
                            }
                        }
                        Variable::Var(v) => {
                            if need_capture && !is_global {
                                let offset = self.get_next_capture_offset();

                                p.to_mut().need_done.push(OpCode::Capture {
                                    stack_offset: *v,
                                    capture_stack_offset: offset,
                                });

                                p.to_mut()
                                    .variables
                                    .insert(name.clone(), Variable::Captured(offset));

                                return OpCode::WriteCapturedVar {
                                    from: from,
                                    offset: offset,
                                };
                            } else if is_global {
                                let runtime = Runtime::current();
                                let id = runtime.to_mut().regester_dynamic_var_name(&format!(
                                    "{}#{}",
                                    name.0.as_ref(),
                                    name.1.as_u32()
                                ));

                                return OpCode::WriteDynamicVar { from: from, id: id };
                            } else {
                                return OpCode::WriteToStack {
                                    from: from,
                                    stack_offset: *v,
                                };
                            }
                        }
                    }
                } else {
                    parent = p.parent.clone();
                }
            } else {
                break;
            }
        }

        let runtime = Runtime::current();
        let id = runtime.regester_dynamic_var_name(&name.0);
        return OpCode::WriteDynamicVar { from: from, id: id };
    }

    pub fn declare(&mut self, name: Id, from: Register, kind: DeclareKind) -> OpCode {
        let off = self.get_next_stack_offset();

        if kind == DeclareKind::Let {
            self.variables.insert(name.clone(), Variable::Let(off));
        } else if kind == DeclareKind::Var {
            self.variables.insert(name.clone(), Variable::Var(off));
        } else if kind == DeclareKind::Const {
            self.variables.insert(name.clone(), Variable::Const(off));
        } else {
            return self.set(name.clone(), from);
        }

        if !self.is_global_context {
            OpCode::WriteToStack {
                from: from,
                stack_offset: off,
            }
        } else {
            let runtime = Runtime::current();
            let id = runtime.regester_dynamic_var_name(name.0.as_ref());

            OpCode::DeclareDynamicVar {
                from: from,
                kind: kind,
                id: id,
            }
        }
    }
}
