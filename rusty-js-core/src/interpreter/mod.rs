use std::collections::HashMap;

use likely_stable::likely;

use crate::bultins::object::JObject;
use crate::bultins::strings::JString;
use crate::bytecodes::{Block, OpCode, Register, TempAllocValue};
use crate::debug;
use crate::error::Error;
use crate::fast_iter::FastIterator;
use crate::operations;
use crate::runtime::Runtime;
use crate::types::JValue;

/// todo: use actual cpu registers to speed up operations
struct Registers([JValue; 3]);

impl std::ops::Index<Register> for Registers {
    type Output = JValue;
    fn index(&self, index: Register) -> &Self::Output {
        &self.0[index.0 as usize]
    }
}

impl std::ops::IndexMut<Register> for Registers {
    fn index_mut(&mut self, index: Register) -> &mut Self::Output {
        &mut self.0[index.0 as usize]
    }
}

pub enum Res {
    Ok,
    Break(Block),
    Return(JValue),
    Err(JValue),
}

pub struct Interpreter<'a> {
    runtime: &'a Runtime,

    r: Registers,

    stack: &'a mut [JValue],
    capture_stack: Option<&'a mut [JValue]>,
    arg_len:usize,

    blocks: HashMap<Block, usize, crate::utils::nohasher::NoHasherBuilder>,
    need_jump: Option<Block>,

    /// in a try statement
    catch_block: Vec<Block>,

    is_global: bool,

    iterators: Vec<&'static mut FastIterator>,
    iter_done: bool,

    temps: Vec<JValue>,
    temp_allocates: Vec<Box<[u8]>>,
}

impl<'a> Interpreter<'a> {
    #[inline]
    pub fn global(runtime: &'a Runtime, stack: &'a mut [JValue]) -> Self {
        Self {
            runtime,
            r: Registers([JValue::UNDEFINED; 3]),
            stack: stack,
            capture_stack: None,
            arg_len:0,
            blocks: Default::default(),
            need_jump: None,
            catch_block: Vec::new(),

            is_global: true,

            iterators: Vec::new(),
            iter_done: false,
            temps: Vec::new(),
            temp_allocates: Vec::new(),
        }
    }

    #[inline]
    pub fn function(
        runtime: &'a Runtime,
        stack: &'a mut [JValue],
        capture_stack: &'a mut [JValue],
    ) -> Self {
        Self {
            runtime,
            r: Registers([JValue::UNDEFINED; 3]),
            stack: stack,
            capture_stack: Some(capture_stack),
            arg_len:0,

            blocks: Default::default(),
            need_jump: None,
            catch_block: Vec::new(),

            is_global: false,

            iterators: Vec::new(),
            iter_done: false,
            temps: Vec::new(),
            temp_allocates: Vec::new(),
        }
    }

    #[inline]
    pub fn run(
        &mut self,
        mut this: JValue,
        args: &[JValue],
        codes: &[OpCode],
    ) -> Result<JValue, JValue> {
        let mut i = 0;

        loop {
            if i == codes.len(){
                break;
            }
            let code = codes[i];

            println!("run code {:#?}", code);
            if self.need_jump.is_none() {
                //debug::debug!("run code {:#?}", code);

                let mut re = Res::Ok;

                if let OpCode::Loop { body_start, body_len } = code{
                    'outer:loop{
                        let mut code_index = body_start as usize;
    
                        while code_index < body_start as usize + body_len as usize{
                            let code = codes[code_index];
        
                            if self.need_jump.is_none() {
                                
                                //debug::debug!("run code {:#?}", code);

                                let r = self.run_code(&mut this, args, code, &mut code_index);
                
                                match r {
                                    Res::Err(_) |
                                    Res::Return(_)=> {
                                        i = code_index;
                                        re = r;
                                        break 'outer;
                                    },
                                    Res::Break(exit) => {
                                        i = body_start as usize + body_len as usize;
                                        self.need_jump = Some(exit);
                                        break 'outer;
                                    }
                                    Res::Ok => {}
                                }
                            } else {
                                //debug::debug!("need jump {:?}", self.need_jump.unwrap());
                
                                if let Some(b) = self.blocks.get(&self.need_jump.unwrap()) {
                                    // block not in loop
                                    let b = *b;
                                    if b < body_start as usize || b >= body_start as usize + body_len as usize{
                                        i = b;
                                        self.need_jump = None;

                                        re = Res::Ok;
                                        break 'outer;
                                    
                                    } else{
                                        // block in loop
                                        code_index = b;
                                        self.need_jump = None;
                                    }
                                    
                
                                } else if let OpCode::SwitchToBlock(b) = code {
                                    self.blocks.insert(b, code_index);
                
                                    //debug::debug!("switch to block {:?}", b);
                                    // jump resolved
                                    if self.need_jump.unwrap() == b {
                                        self.need_jump = None;
                                    }
                
                                    // jump not resolved, loop until next switch to block
                                }
                            };
                            code_index += 1;
                        };
                    };
                } else{
                    re = self.run_code(&mut this, args, code, &mut i);
                };

                match re {
                    Res::Err(e) => {
                        if self.catch_block.len() > 0 {
                            self.r[Register(0)] = e;
                            self.need_jump = Some(*self.catch_block.last().unwrap());
                        } else {
                            return Err(e);
                        }
                    },
                    // the outermost context should not be break
                    Res::Break(_) => {},
                    Res::Ok => {}
                    Res::Return(r) => return Ok(r),
                }
            } else {
                //debug::debug!("need jump {:?}", self.need_jump.unwrap());

                if let Some(b) = self.blocks.get(&self.need_jump.unwrap()) {
                    // jump resolved
                    i = *b;
                    self.need_jump = None;

                } else if let OpCode::SwitchToBlock(b) = code {
                    self.blocks.insert(b, i);

                    //debug::debug!("switch to block {:?}", b);
                    // jump resolved
                    if self.need_jump.unwrap() == b {
                        self.need_jump = None;
                    }

                    // jump not resolved, loop until next switch to block
                }
            }

            i += 1;
        }
        Ok(JValue::UNDEFINED)
    }

    #[inline]
    fn run_code(
        &mut self,
        this: &mut JValue,
        args: &[JValue],
        code: OpCode,
        index: &mut usize,
    ) -> Res {
        match code {
            OpCode::NoOp => {}
            OpCode::Debugger => {},
            OpCode::Loop { body_start:_, body_len:_ } => {
                // stack overflow
                return Res::Ok; 
            },
            OpCode::Break{ exit } => {
                //return Res::Ok;
                return Res::Break(exit);
            },
            OpCode::BreakIfTrue { value, exit } => {
                //return Res::Ok;
                if self.r[value].to_bool(){
                    return Res::Break(exit)
                }
            },
            OpCode::BreakIfFalse { value, exit } => {
                //return Res::Ok;
                if !self.r[value].to_bool(){
                    return Res::Break(exit)
                }
            },
            OpCode::BreakIfIterDone{ exit } => {
                //return Res::Ok;
                if self.iter_done{
                    return Res::Break(exit);
                }
            }
            OpCode::Add {
                result,
                left,
                right,
            } => {

                let lhs = self.r[left];
                let rhs = self.r[right];
                if likely(lhs.is_number() && rhs.is_number()){
                    unsafe{self.r[result] = JValue::Number(lhs.value.number + rhs.value.number)};
                } else{
                    self.r[result] = lhs + rhs;
                }
            }
            OpCode::Sub {
                result,
                left,
                right,
            } => {
                self.r[result] = self.r[left] - self.r[right];
            }
            OpCode::Mul {
                result,
                left,
                right,
            } => {
                self.r[result] = self.r[left] * self.r[right];
            }
            OpCode::Div {
                result,
                left,
                right,
            } => {
                self.r[result] = self.r[left] / self.r[right];
            }
            OpCode::Rem {
                result,
                left,
                right,
            } => {
                self.r[result] = self.r[left] % self.r[right];
            }
            OpCode::And {
                result,
                left,
                right,
            } => {
                self.r[result] = (self.r[left].to_bool() && self.r[right].to_bool()).into();
            }
            OpCode::Or {
                result,
                left,
                right,
            } => {
                if self.r[left].to_bool() {
                    self.r[result] = self.r[left];
                } else {
                    self.r[result] = self.r[right];
                }
            }
            OpCode::Not { result, right } => {
                self.r[result] = (!self.r[right].to_bool()).into();
            }
            OpCode::Exp {
                result,
                left,
                right,
            } => {
                let (v, error) = unsafe { self.r[left].exp_(self.r[right]) };
                if error {
                    return Res::Err(v);
                }
                self.r[result] = v;
            }
            OpCode::Plus { result, right } => {
                self.r[result] = self.r[right].to_number().into();
            }
            OpCode::Minus { result, right } => {
                self.r[result] = (-self.r[right].to_number()).into();
            }
            OpCode::LShift {
                result,
                left,
                right,
            } => {
                self.r[result] = self.r[left] << self.r[right];
            }
            OpCode::RShift {
                result,
                left,
                right,
            } => {
                self.r[result] = self.r[left] >> self.r[right];
            }
            OpCode::ZeroFillRShift {
                result,
                left,
                right,
            } => {
                self.r[result] = self.r[left].zerofillRshift(self.r[right]);
            }
            OpCode::Gt {
                result,
                left,
                right,
            } => self.r[result] = (self.r[left] > self.r[right]).into(),
            OpCode::GtEq {
                result,
                left,
                right,
            } => self.r[result] = (self.r[left] >= self.r[right]).into(),
            OpCode::Lt {
                result,
                left,
                right,
            } => self.r[result] = (self.r[left] < self.r[right]).into(),
            OpCode::LtEq {
                result,
                left,
                right,
            } => self.r[result] = (self.r[left] <= self.r[right]).into(),
            OpCode::EqEq {
                result,
                left,
                right,
            } => self.r[result] = unsafe { self.r[left].eqeq_(self.r[right]) },
            OpCode::EqEqEq {
                result,
                left,
                right,
            } => {
                self.r[result] = unsafe {
                    std::mem::transmute::<_, [usize; 2]>(self.r[left])
                        == std::mem::transmute::<_, [usize; 2]>(self.r[right])
                }
                .into()
            }
            OpCode::NotEq {
                result,
                left,
                right,
            } => self.r[result] = unsafe { !self.r[left].eqeq_(self.r[right]) },
            OpCode::NotEqEq {
                result,
                left,
                right,
            } => {
                self.r[result] = unsafe {
                    !(std::mem::transmute::<_, [usize; 2]>(self.r[left])
                        == std::mem::transmute::<_, [usize; 2]>(self.r[right]))
                }
                .into()
            }
            OpCode::BitAnd {
                result,
                left,
                right,
            } => {
                self.r[result] = self.r[left] & self.r[right];
            }
            OpCode::BitNot { result, right } => {
                self.r[result] = self.r[right].bitnot();
            }
            OpCode::BitOr {
                result,
                left,
                right,
            } => {
                self.r[result] = self.r[left] | self.r[right];
            }
            OpCode::BitXor {
                result,
                left,
                right,
            } => {
                self.r[result] = self.r[left] ^ self.r[right];
            }
            OpCode::Await { result, future } => {
                let (v, error) = if self.is_global {
                    self.r[future].wait()
                } else {
                    operations::async_wait(self.r[future])
                };

                if error {
                    return Res::Err(v);
                }
                self.r[result] = v;
            }
            OpCode::Yield { result, arg } => {
                self.r[result] = operations::Yield(self.r[arg]);
            }
            OpCode::In {
                result,
                left,
                right,
            } => {
                let (v, error) = unsafe { self.r[left].In_(self.r[right]) };
                if error {
                    return Res::Err(v);
                }
                self.r[result] = v;
            }
            OpCode::PrivateIn {
                result,
                name,
                right,
            } => {
                self.r[result] = unsafe { self.r[right].private_in(name) };
            }
            OpCode::InstanceOf {
                result,
                left,
                right,
            } => {
                let (v, error) = unsafe { self.r[left].instance_of_(self.r[right]) };
                if error {
                    return Res::Err(v);
                }
                self.r[result] = v;
            }
            OpCode::TypeOf { result, right } => {
                let s = self.r[right].type_str();
                self.r[result] = JValue::String(JString::from_static(s));
            }
            OpCode::Select { a, b, result } => {
                if self.r[a].is_undefined() {
                    self.r[result] = self.r[b];
                } else {
                    self.r[result] = self.r[a];
                }
            }
            OpCode::CondSelect { t, a, b, result } => {
                if self.r[t].to_bool() {
                    self.r[result] = self.r[a];
                } else {
                    self.r[result] = self.r[b];
                }
            }
            OpCode::Nullish {
                result,
                left,
                right,
            } => {
                let l = self.r[left];
                if l.is_undefined() || l.is_null() {
                    self.r[result] = self.r[right];
                } else {
                    self.r[result] = self.r[left];
                }
            }

            OpCode::Throw { value } => return Res::Err(self.r[value]),
            OpCode::Return { value } => return Res::Return(self.r[value]),

            ///////////////////////////////////////////////////////////////////////
            //   iterator
            //////////////////////////////////////////////////////////////////////
            OpCode::IntoIter { target, hint } => {
                let iter = unsafe { FastIterator::new(self.r[target], hint) };
                self.iterators.push(iter);
            }
            OpCode::IterDrop => {
                let iter = self.iterators.pop().unwrap();
                FastIterator::drop_(iter);
            }
            OpCode::IterNext {
                result,
                hint: _,
                stack_offset,
            } => {
                let iter = self.iterators.last_mut().unwrap();

                let stack = &mut self.stack[stack_offset as usize..];

                let (done, error, value) = iter.next(*this, stack.as_mut_ptr());
                if error {
                    return Res::Err(value);
                }
                self.iter_done = done;
                self.r[result] = value;
            }
            OpCode::IterCollect {
                result,
                stack_offset,
            } => {
                let iter = self.iterators.last_mut().unwrap();

                let stack = &mut self.stack[stack_offset as usize..];

                let (v, error) = iter.collect(*this, stack.as_mut_ptr());
                if error {
                    return Res::Err(v);
                }
                self.r[result] = v;
            }

            //////////////////////////////////////////////////////////////////
            //        memory
            //////////////////////////////////////////////////////////////////
            OpCode::Mov { from, to } => {
                self.r[to] = self.r[from];
            }
            OpCode::StoreTemp { value } => {
                self.temps.push(self.r[value]);
            }
            OpCode::ReadTemp { value } => {
                self.r[value] = *self.temps.last().unwrap();
            }
            OpCode::ReleaseTemp => {
                self.temps.pop();
            }
            OpCode::WriteToStack { from, stack_offset } => {
                self.stack[stack_offset as usize] = self.r[from];
            }
            OpCode::ReadFromStack {
                result,
                stack_offset,
            } => {
                self.r[result] = self.stack[stack_offset as usize];
            }

            OpCode::DeclareDynamicVar { from, kind: _, id } => {
                self.runtime.declare_variable_static(id, self.r[from]);
            }
            OpCode::WriteDynamicVar { from, id } => {
                self.runtime.to_mut().set_variable(id, self.r[from]);
            }
            OpCode::ReadDynamicVar { result, id } => {
                let (v, error) = self.runtime.to_mut().get_variable(id);
                if error {
                    return Res::Err(v);
                }
                self.r[result] = v;
            }

            OpCode::Capture {
                stack_offset,
                capture_stack_offset,
            } => {
                if let Some(c) = &mut self.capture_stack {
                    c[capture_stack_offset as usize] = self.stack[stack_offset as usize];
                } else {
                    panic!("capture variable on global context")
                }
            }
            OpCode::ReadCapturedVar { result, offset } => {
                if let Some(c) = &mut self.capture_stack {
                    self.r[result] = c[offset as usize];
                } else {
                    panic!("reading capture variable on global context")
                }
            }
            OpCode::WriteCapturedVar { from, offset } => {
                if let Some(c) = &mut self.capture_stack {
                    c[offset as usize] = self.r[from];
                } else {
                    panic!("capture variable on global context")
                }
            }

            OpCode::TempAlloc { size } => {
                let mut v = Vec::with_capacity(size as usize);
                v.resize(size as usize, 0u8);
                self.temp_allocates.push(unsafe { Box::from_raw(v.leak()) });
            }
            OpCode::TempDealloc { size: _ } => {
                self.temp_allocates.pop();
            }
            OpCode::StoreTempAlloc {
                offset,
                flag,
                value,
            } => {
                let s = &mut self.temp_allocates.last_mut().unwrap()[offset as usize..];
                let v = s.as_mut_ptr() as *mut TempAllocValue;
                unsafe {
                    *v = TempAllocValue {
                        flag,
                        value: self.r[value],
                    }
                };
            }
            OpCode::ReadTempAlloc { offset, result } => {
                let s = &mut self.temp_allocates.last_mut().unwrap()[offset as usize..];
                let v = unsafe { (s.as_mut_ptr() as *mut TempAllocValue).as_mut().unwrap() };
                self.r[result] = v.value;
            }
            OpCode::SetThis { value } => {
                *this = self.r[value];
            }

            ////////////////////////////////////////////////////////////////
            //            function
            ////////////////////////////////////////////////////////////////
            OpCode::CreateArg {
                stack_offset:_,
                len: _,
            } => {

            },
            OpCode::PushArg {
                value,
                stack_offset,
            } => {
                self.stack[stack_offset as usize] = self.r[value];
            }
            OpCode::PushArgSpread {
                value,
                stack_offset,
            } => {
                self.stack[stack_offset as usize] = self.r[value].spread();
            }
            OpCode::FinishArgs {
                base_stack_offset,
                len,
            } => {
                //todo: unknown bug
                let mut len = len as usize;
                let mut i = base_stack_offset as usize;

                while i < base_stack_offset as usize + len {

                    let v = self.stack[i as usize];

                    if v.is_spread(){
                        let mut values = vec![];
                        
                        let iter =
                            unsafe { FastIterator::new(v, crate::bytecodes::LoopHint::For) };
                        loop {
                            let offset = base_stack_offset as usize + len;
                            let stack = &mut self.stack[offset..];

                            let (done, error, value) = iter.next(*this, stack.as_mut_ptr());
                            if error {
                                return Res::Err(value);
                            }
                            values.push(value);
                            if done {
                                break;
                            }
                        };

                        FastIterator::drop_(iter);
                        len  += values.len() -1;
                        i += values.len()-1;

                        unsafe{
                            let p = self.stack.as_mut_ptr().add(i);
                            std::ptr::copy(p, p.add(values.len()), values.len());
                            std::ptr::copy(values.as_ptr(), p, values.len());
                        };
                    }
                    i += 1;
                };
                self.arg_len = len as usize;
            }

            OpCode::ReadParam { result, index } => {
                if index as usize >= args.len() {
                    self.r[result] = JValue::UNDEFINED;
                } else {
                    self.r[result] = args[index as usize];
                }
            }
            OpCode::CollectParam { result, start } => {
                let obj = JObject::array();

                if (start as usize) < args.len() {
                    for i in &args[start as usize..] {
                        obj.as_array().unwrap().push((Default::default(), *i));
                    }
                }
                self.r[result] = JValue::Object(obj);
            }

            OpCode::Call {
                result,
                this,
                callee,
                stack_offset,
            } => {
                let stack = &mut self.stack[stack_offset as usize..];
                let (v, error) = unsafe {
                    self.r[callee].call_raw(
                        self.runtime,
                        self.r[this],
                        stack.as_mut_ptr(),
                        self.arg_len as u32,
                    )
                };
                if error {
                    return Res::Err(v);
                }
                self.r[result] = v;
            }
            OpCode::CallOptChain {
                result,
                this,
                callee,
                stack_offset,
            } => {
                if self.r[callee].is_null() || self.r[callee].is_undefined() {
                    self.r[result] = JValue::UNDEFINED;
                } else {
                    let stack = &mut self.stack[stack_offset as usize..];
                    let (v, error) = unsafe {
                        self.r[callee].call_raw(
                            self.runtime,
                            self.r[this],
                            stack.as_mut_ptr(),
                            self.arg_len as u32,
                        )
                    };
                    if error {
                        return Res::Err(v);
                    }
                    self.r[result] = v;
                }
            }
            OpCode::New {
                result,
                callee,
                stack_offset,
            } => {
                let stack = &mut self.stack[stack_offset as usize..];

                let constructor = self.r[callee];
                let (v, error) = operations::invoke_new(
                    constructor,
                    self.runtime,
                    stack.as_mut_ptr(),
                    self.arg_len as u32,
                );
                if error {
                    return Res::Err(v);
                }
                self.r[result] = v;
            }
            OpCode::NewTarget { result } => {
                self.r[result] = operations::new_target(&self.runtime);
            }
            OpCode::ImportMeta { result } => {
                self.r[result] = operations::import_meta(&self.runtime);
            }

            ////////////////////////////////////////////////////////////////////////
            //             blocks
            ////////////////////////////////////////////////////////////////////////
            OpCode::CreateBlock(_b) => {
                //self.blocks.insert(b, None);
            },
            OpCode::SwitchToBlock(b) => {
                if !self.blocks.contains_key(&b){
                    self.blocks.insert(b, *index);
                }
            },
            OpCode::Jump { to, line:_ } => {
                self.need_jump = Some(to);
            },
            OpCode::JumpIfFalse { value, to } => {
                if !self.r[value].to_bool() {
                    self.need_jump = Some(to);
                }
            },
            OpCode::JumpIfIterDone { to } => {
                if self.iter_done {
                    self.iter_done = false;
                    self.need_jump = Some(to);
                }
            },
            OpCode::JumpIfTrue { value, to } => {
                if self.r[value].to_bool() {
                    self.need_jump = Some(to);
                }
            },
            OpCode::EnterTry { catch_block } => {
                self.catch_block.push(catch_block);
            }
            OpCode::ExitTry => {
                self.catch_block.pop();
            }

            ////////////////////////////////////////////////////////////////
            //             statics
            ////////////////////////////////////////////////////////////////
            OpCode::LoadFalse { result } => {
                self.r[result] = JValue::FALSE;
            }
            OpCode::LoadTrue { result } => {
                self.r[result] = JValue::TRUE;
            }
            OpCode::LoadNull { result } => {
                self.r[result] = JValue::NULL;
            }
            OpCode::LoadUndefined { result } => {
                self.r[result] = JValue::UNDEFINED;
            }
            OpCode::LoadThis { result } => {
                self.r[result] = *this;
            }
            OpCode::LoadStaticBigInt { result, id } => {
                self.r[result] = self.runtime.get_unamed_constant(id);
            }
            OpCode::LoadStaticBigInt32 { result, value } => {
                self.r[result] = JValue::BigInt(value as i64);
            }
            OpCode::LoadStaticFloat { result, id } => {
                self.r[result] = self.runtime.get_unamed_constant(id);
            }
            OpCode::LoadStaticFloat32 { result, value } => {
                self.r[result] = JValue::Number(value as f64);
            }
            OpCode::LoadStaticString { result, id } => {
                let s = self.runtime.get_string(id);
                self.r[result] = JValue::String(JString::from_static(s));
            }

            //////////////////////////////////////////////////////////////////
            //                 object
            //////////////////////////////////////////////////////////////////
            OpCode::ReadField {
                obj,
                field,
                result,
                stack_offset,
            } => {
                let obj = self.r[obj];
                let field = self.r[field].to_string();
                let id = self.runtime.register_field_name(&field);
                let stack = &mut self.stack[stack_offset as usize..];

                let (re, error) = obj.get_property_raw(id, stack.as_mut_ptr());

                if error {
                    return Res::Err(re);
                }
                self.r[result] = re;
            }
            OpCode::ReadFieldOptChain {
                obj,
                field,
                result,
                stack_offset,
            } => {
                let obj = self.r[obj];

                if obj.is_null() || obj.is_undefined() {
                    self.r[result] = JValue::UNDEFINED;
                } else {
                    let field = self.r[field].to_string();
                    let id = self.runtime.register_field_name(&field);
                    let stack = &mut self.stack[stack_offset as usize..];

                    let (re, error) = obj.get_property_raw(id, stack.as_mut_ptr());

                    if error {
                        return Res::Err(re);
                    }
                    self.r[result] = re;
                }
            }
            OpCode::ReadFieldStatic {
                obj_result,
                field_id,
                stack_offset,
            } => {
                let obj = self.r[obj_result.target()];
                let stack = &mut self.stack[stack_offset as usize..];

                let (re, error) = obj.get_property_raw(field_id, stack.as_mut_ptr());

                if error {
                    return Res::Err(re);
                }
                self.r[obj_result.value()] = re;
            }
            OpCode::ReadFieldStaticOptChain {
                obj_result,
                field_id,
                stack_offset,
            } => {
                let obj = self.r[obj_result.target()];

                if obj.is_null() || obj.is_undefined() {
                    self.r[obj_result.value()] = JValue::UNDEFINED;
                } else {
                    let stack = &mut self.stack[stack_offset as usize..];

                    let (re, error) = obj.get_property_raw(field_id, stack.as_mut_ptr());

                    if error {
                        return Res::Err(re);
                    }
                    self.r[obj_result.value()] = re;
                }
            }

            OpCode::WriteField {
                obj,
                field,
                value,
                stack_offset,
            } => {
                let obj = self.r[obj];
                let field = self.r[field].to_string();
                let id = self.runtime.register_field_name(&field);
                let stack = &mut self.stack[stack_offset as usize..];

                let (re, error) = obj.set_property_raw(id, self.r[value], stack.as_mut_ptr());

                if error {
                    return Res::Err(re);
                }
            }
            OpCode::WriteFieldOptChain {
                obj,
                field,
                from,
                stack_offset,
            } => {
                let obj = self.r[obj];
                if obj.is_null() || obj.is_undefined() {
                } else {
                    let field = self.r[field].to_string();
                    let id = self.runtime.register_field_name(&field);
                    let stack = &mut self.stack[stack_offset as usize..];

                    let (re, error) = obj.set_property_raw(id, self.r[from], stack.as_mut_ptr());

                    if error {
                        return Res::Err(re);
                    }
                }
            }
            OpCode::WriteFieldStatic {
                obj_value,
                field_id,
                stack_offset,
            } => {
                let obj = self.r[obj_value.target()];
                let value = self.r[obj_value.value()];

                let stack = &mut self.stack[stack_offset as usize..];

                let (re, error) = obj.set_property_raw(field_id, value, stack.as_mut_ptr());

                if error {
                    return Res::Err(re);
                }
            }

            OpCode::RemoveFieldStatic { obj, field_id } => {
                let obj = self.r[obj];
                if obj.is_object() {
                    unsafe { obj.remove_key_static_(field_id) }
                }
            }

            OpCode::ReadSuperField {
                constructor_result,
                field,
                stack_offset,
            } => {
                let stack = &mut self.stack[stack_offset as usize..];
                let (v, err) = unsafe {
                    operations::super_prop(
                        &self.runtime,
                        self.r[constructor_result.target()],
                        self.r[field],
                        stack.as_mut_ptr(),
                    )
                };
                if err {
                    return Res::Err(v);
                }
                self.r[constructor_result.value()] = v;
            }
            OpCode::ReadSuperFieldStatic {
                constructor_result,
                field_id,
                stack_offset,
            } => {
                let stack = &mut self.stack[stack_offset as usize..];
                let (v, err) = unsafe {
                    operations::super_prop_static(
                        &self.runtime,
                        self.r[constructor_result.target()],
                        field_id,
                        stack.as_mut_ptr(),
                    )
                };
                if err {
                    return Res::Err(v);
                }
                self.r[constructor_result.value()] = v;
            }
            OpCode::WriteSuperField {
                constructor_value,
                field,
                stack_offset,
            } => {
                let stack = &mut self.stack[stack_offset as usize..];
                let (v, err) = unsafe {
                    operations::super_write_prop(
                        &self.runtime,
                        self.r[constructor_value.target()],
                        self.r[field],
                        self.r[constructor_value.value()],
                        stack.as_mut_ptr(),
                    )
                };
                if err {
                    return Res::Err(v);
                };
            }
            OpCode::WriteSuperFieldStatic {
                constructor_value,
                field,
                stack_offset,
            } => {
                let stack = &mut self.stack[stack_offset as usize..];
                let (v, err) = unsafe {
                    operations::super_write_prop_static(
                        &self.runtime,
                        self.r[constructor_value.target()],
                        field,
                        self.r[constructor_value.value()],
                        stack.as_mut_ptr(),
                    )
                };
                if err {
                    return Res::Err(v);
                };
            }

            OpCode::BindGetter {
                obj,
                field_id,
                getter,
            } => {
                let obj = self.r[obj];
                if obj.is_object() {
                    unsafe {
                        obj.value
                            .object
                            .bind_getter(field_id, self.r[getter].value.object)
                    }
                }
            }
            OpCode::BindSetter {
                obj,
                field_id,
                setter,
            } => {
                let obj = self.r[obj];
                if obj.is_object() {
                    unsafe {
                        obj.value
                            .object
                            .bind_setter(field_id, self.r[setter].value.object)
                    }
                }
            }
            OpCode::CloneObject { obj, result } => {
                let obj = self.r[obj];
                if obj.is_object() {
                    self.r[result] = JValue::Object(unsafe { obj.value.object.deep_clone() });
                }
            }
            OpCode::ExtendObject { obj, from } => {
                unsafe { operations::extend_object(self.r[obj], self.r[from]) };
            }

            ////////////////////////////////////////////////////////////////
            //           creations
            ///////////////////////////////////////////////////////////////
            OpCode::CreateArray { result } => {
                self.r[result] = JValue::Object(JObject::array());
            }
            OpCode::CreateArrow { result, this, id } => {
                let f = if self.is_global {
                    self.runtime
                        .get_function(id)
                        .unwrap()
                        .create_instance(Some(self.r[this]))
                } else {
                    let ptr = self.capture_stack.as_mut().unwrap().as_mut_ptr();
                    self.runtime
                        .get_function(id)
                        .unwrap()
                        .create_instance_with_capture(Some(self.r[this]), ptr)
                };

                self.r[result] = JValue::Object(JObject::with_function(f));
            }
            OpCode::CreateFunction { result, id } => {
                let f = if self.is_global {
                    self.runtime.get_function(id).unwrap().create_instance(None)
                } else {
                    self.runtime
                        .get_function(id)
                        .unwrap()
                        .create_instance_with_capture(
                            None,
                            self.capture_stack.as_mut().unwrap().as_mut_ptr(),
                        )
                };

                self.r[result] = JValue::Object(JObject::with_function(f));
            }

            OpCode::CreateObject { result } => {
                self.r[result] = JValue::Object(JObject::new());
            }
            OpCode::CreateRegExp { result, reg_id } => {
                let r = self.runtime.get_regex(reg_id);
                self.r[result] = JObject::with_regex(r).into();
            }
            OpCode::CreateTemplate { result, id, stack_offset } => {
                let stack = &mut self.stack[stack_offset as usize..];

                self.r[result] = unsafe {
                    operations::create_template(
                        id.0,
                        stack.as_mut_ptr(),
                        self.arg_len as u32,
                        false,
                    )
                };
            },
            OpCode::CreateTaggedTemplate { result, id, stack_offset } => {
                let stack = &mut self.stack[stack_offset as usize..];

                self.r[result] = unsafe {
                    operations::create_template(
                        id.0,
                        stack.as_mut_ptr(),
                        self.arg_len as u32,
                        true,
                    )
                };
            },

            OpCode::CreateClass { result, class_id } => {
                let class = self.runtime.get_class(class_id);

                let obj = if self.is_global {
                    class.create_object_without_capture()
                } else {
                    class.create_object_with_capture(
                        self.capture_stack.as_mut().unwrap().as_mut_ptr(),
                    )
                };

                self.r[result] = obj.into();
            },
            OpCode::ClassBindSuper { class, super_ } => {
                let super_class = self.r[super_];
                let c = self.r[class];
                let proto = c.get_property_str("prototype").unwrap();
                let super_proto = super_class.get_property_str("prototype").unwrap();
                proto.set_property_str("__proto__", super_proto).unwrap();

                if !super_class.is_object() {
                    return Res::Err(JValue::Error(Error::ClassExtendsNonCallable));
                }
                if c.is_object() {
                    if let Some(c) = unsafe { c.value.object.as_class() } {
                        c.super_ = Some(unsafe { super_class.value.object })
                    }
                }
            }
        };
        Res::Ok
    }
}
