use std::sync::Arc;

use futures::Future;
use fxhash::FxHashMap;
use likely_stable::likely;

use crate::bultins::flag::PropFlag;
use crate::bultins::function::CaptureStack;
use crate::bultins::object::JObject;
use crate::bytecodes::{Block, OpCode, Register};
use crate::error::Error;
use crate::runtime::Runtime;
use crate::types::JValue;
use crate::utils::iterator::JSIterator;
use crate::utils::string_interner::NAMES;
use crate::{operations, JSContext, Promise, PropKey, ToProperyKey};

//use self::block_compiler::CompiledBlock;

mod block_compiler;
pub mod clousure;
//pub mod clousure1;
//pub use clousure1 as clousure;

/// todo: use actual cpu registers to speed up operations
#[repr(transparent)]
struct Registers([JValue; 3]);

impl Registers {
    #[inline]
    pub fn read(&self, reg: Register) -> JValue {
        unsafe { *self.0.get_unchecked(reg.0 as usize) }
    }

    #[inline]
    pub fn write(&self, reg: Register, value: JValue) {
        unsafe {
            std::ptr::write_volatile(
                (self as *const Self as *mut JValue).add(reg.0 as usize),
                value,
            );
        }
    }
}

impl std::ops::Index<Register> for Registers {
    type Output = JValue;
    fn index(&self, index: Register) -> &Self::Output {
        unsafe { self.0.get_unchecked(index.0 as usize) }
    }
}

impl std::ops::IndexMut<Register> for Registers {
    fn index_mut(&mut self, index: Register) -> &mut Self::Output {
        unsafe { self.0.get_unchecked_mut(index.0 as usize) }
    }
}

pub enum Res {
    Ok,
    Await(JValue, Register),
    Yield(JValue, Register),

    Return(JValue),

    Err(JValue),
}

pub struct Interpreter<'a> {
    runtime: &'a Runtime,

    r: Registers,

    stack: &'a mut [JValue],
    op_stack_offset: usize,
    cap: Option<CaptureStack>,
    capture_stack: Option<&'a mut [JValue]>,

    arg_offset_counter: i64,
    arg_len: usize,

    //clousure_blocks: Vec<Option<Box<dyn Fn(
    //    &mut Interpreter,
    //    JSContext,
    //    &mut Registers,
    //    &mut JValue,
    //    &[JValue],
    //    &mut [JValue],
    //    &mut usize,
    //) -> Result<Res, JValue>>>>,

    blocks: Vec<Option<(u64, usize)>>,
    compiled_blocks: FxHashMap<Block, block_compiler::CompiledBlock>,
    call_sites: FxHashMap<usize, (JValue, usize)>,

    /// in a try statement
    catch_block: Vec<(Block, u32)>,

    is_global: bool,

    iterators: Vec<JSIterator<'a>>,
    iter_done: bool,

    temps: Vec<JValue>,
    temp_allocates: Vec<Box<[u8]>>,
}

impl<'a> Interpreter<'a> {
    #[inline]
    pub fn global(runtime: &'a Runtime, stack: &'a mut [JValue], op_stack_offset: usize) -> Self {
        Self {
            runtime,
            r: Registers([JValue::UNDEFINED; 3]),
            stack: stack,
            op_stack_offset: op_stack_offset,
            cap: None,
            capture_stack: None,
            arg_offset_counter: 0,
            arg_len: 0,

            blocks: Default::default(),
            compiled_blocks: Default::default(),

            call_sites: Default::default(),
            //need_jump: None,
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
        op_stack_offset: usize,
        capture_stack: CaptureStack,
        capture_stack_data: Option<&'a mut [JValue]>,
    ) -> Self {
        Self {
            runtime,
            r: Registers([JValue::UNDEFINED; 3]),
            stack: stack,
            op_stack_offset,
            cap: Some(capture_stack),
            capture_stack: capture_stack_data,

            arg_offset_counter: 0,
            arg_len: 0,

            blocks: Default::default(),
            compiled_blocks: Default::default(),

            call_sites: Default::default(),

            //need_jump: None,
            catch_block: Vec::new(),

            is_global: false,

            iterators: Vec::new(),
            iter_done: false,
            temps: Vec::new(),
            temp_allocates: Vec::new(),
        }
    }

    #[inline]
    fn insert_block(&mut self, block: Block, index: usize) {
        if let Some(b) = self.blocks.get_mut(block.0 as usize) {
            if b.is_none() {
                *b = Some((0, index));
            }
        } else {
            self.blocks.resize(block.0 as usize + 2, None);
            self.blocks[block.0 as usize] = Some((0, index))
        }
    }

    #[inline]
    fn compile_block(&mut self, block: Block, bytecodes: &[OpCode]) {
        if self.is_global {
            let start = self.blocks[block.0 as usize].unwrap().1 + 1;
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
            if i == codes.len() {
                break;
            }
            let code = codes[i];

            //debug::debug!("run code {:#?}", code);
            
            let intpr:&mut Self = unsafe{std::mem::transmute_copy(&self)};

            let re = self.run_code(&mut this, args, code, codes, &mut i);

            match re {
                Err(e) => {
                    if let Some((_catch_block, line)) = self.catch_block.pop() {
                        self.r[Register(0)] = e;

                        i = line as usize;
                    } else {
                        return Err(e);
                    }
                }
                Ok(re) => {
                    match re {
                        Res::Await(_, _) => {}
                        Res::Yield(_, _) => {}
                        Res::Ok => {}
                        Res::Err(e) => {
                            if let Some((_catch_block, line)) = self.catch_block.pop() {
                                self.r[Register(0)] = e;

                                i = line as usize;
                            } else {
                                return Err(e);
                            }
                        }
                        Res::Return(r) => return Ok(r),
                    }
                }
            }

            i += 1;
        }
        Ok(JValue::UNDEFINED)
    }

    // consume self and create a future
    pub fn run_async(
        self,
        mut this: JValue,
        args: &[JValue],
        codes: Arc<Vec<OpCode>>,
    ) -> impl Future<Output = Result<JValue, JValue>> + 'static {
        let args = args.to_vec();
        let mut intpr: Interpreter<'static> = unsafe { std::mem::transmute(self) };

        async move {
            let mut i = 0;

            loop {
                if i == codes.len() {
                    break;
                }
                let code = codes[i];

                //debug::debug!("run code {:#?}", code);
                let re = intpr.run_code(&mut this, &args, code, &codes, &mut i);

                match re {
                    Err(e) => {
                        if let Some((catch_block, line)) = intpr.catch_block.pop() {
                            intpr.r[Register(0)] = e;

                            i = line as usize;
                        } else {
                            return Err(e);
                        }
                    }
                    Ok(r) => {
                        match r {
                            Res::Await(f, result) => {
                                if let Some(obj) = f.as_object() {
                                    if let Some(p) = obj.as_promise() {
                                        match p {
                                            Promise::ForeverPending => {
                                                return Err(
                                                    Error::AwaitOnForeverPendingPromise.into()
                                                )
                                            }
                                            Promise::Fulfilled(f) => {
                                                intpr.r[result] = *f;
                                            }
                                            Promise::Pending { id } => {
                                                let re =
                                                    intpr.runtime.to_mut().get_future(*id).await;
                                                match re {
                                                    Ok(v) => {
                                                        intpr.r[result] = v;
                                                        *p = Promise::Fulfilled(v);
                                                    }
                                                    Err(e) => {
                                                        *p = Promise::Rejected(e);
                                                        return Err(e);
                                                    }
                                                };
                                            }
                                            Promise::Rejected(e) => return Err(*e),
                                        }
                                    };
                                } else {
                                    intpr.r[result] = f;
                                };
                            }
                            Res::Yield(_, _) => {}
                            // the outermost context should not be break
                            Res::Ok => {}
                            Res::Return(r) => return Ok(r),
                            _ => {}
                        }
                    }
                }

                i += 1;
            }
            Ok(JValue::UNDEFINED)
        }
    }

    pub fn run_async_generator(
        mut self,
        mut this: JValue,
        args: &[JValue],
        codes: &[OpCode],
    ) -> Result<JValue, JValue> {
        let mut i = 0;

        loop {
            if i == codes.len() {
                break;
            }
            let code = codes[i];

            //debug::debug!("run code {:#?}", code);

            let re = self.run_code(&mut this, args, code, codes, &mut i);

            match re {
                Err(e) => {
                    if let Some((catch_block, line)) = self.catch_block.pop() {
                        self.r[Register(0)] = e;

                        i = line as usize;
                    } else {
                        return Err(e);
                    }
                }
                Ok(re) => {
                    match re {
                        Res::Ok => {}
                        Res::Return(r) => return Ok(r),
                        _ => {}
                    }
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
        codes: &[OpCode],
        index: &mut usize,
    ) -> Result<Res, JValue> {
        let ctx = JSContext {
            stack: (&mut self.stack[self.op_stack_offset..]).as_mut_ptr(),
            runtime: self.runtime,
        };

        //println!("{:?}", code);
        match code {
            OpCode::NoOp => {}
            OpCode::Debugger => {}
            OpCode::IsNullish { result, value } => {
                let v = self.r[value];
                self.r[result] = (v.is_null() || v.is_undefined()).into();
            }
            OpCode::Add {
                result,
                left,
                right,
            } => {
                let lhs = self.r[left];
                let rhs = self.r[right];

                if likely(lhs.is_number() && rhs.is_number()) {
                    self.r[result] =
                        JValue::create_number(lhs.as_number_uncheck() + rhs.as_number_uncheck());
                } else {
                    self.r[result] = lhs.add(rhs, ctx)?;
                }
            }
            OpCode::AddImmI32 {
                result,
                left,
                right,
            } => {
                let lhs = self.r[left];
                if likely(lhs.is_number()) {
                    self.r[result] = (lhs.as_number_uncheck() + right as f64).into();
                } else if likely(lhs.is_int()) {
                    self.r[result] = (lhs.as_int_unchecked() + right as i32).into();
                } else {
                    self.r[result] = (lhs.add((right as f64).into(), ctx))?;
                }
            }
            OpCode::AddImmF32 {
                result,
                left,
                right,
            } => {
                let lhs = self.r[left];
                if likely(lhs.is_number()) {
                    self.r[result] = unsafe { lhs.as_number_uncheck() + right as f64 }.into();
                } else {
                    self.r[result] = (lhs.add((right as f64).into(), ctx))?;
                }
            }
            OpCode::AddImmStr { result, left, str } => {
                let lhs = self.r[left];
                self.r[result] = (lhs.to_string() + self.runtime.get_string(str)).into();
            }
            OpCode::Sub {
                result,
                left,
                right,
            } => {
                let lhs = self.r[left];
                let rhs = self.r[right];

                if likely(lhs.is_number() && rhs.is_number()) {
                    self.r[result] = unsafe {
                        JValue::create_number(lhs.as_number_uncheck() - rhs.as_number_uncheck())
                    };
                } else {
                    self.r[result] = lhs.sub(rhs, ctx)?;
                }
            }
            OpCode::SubImmI32 {
                result,
                left,
                right,
            } => {
                let lhs = self.r[left];
                if likely(lhs.is_number()) {
                    self.r[result] = unsafe { lhs.as_number_uncheck() - right as f64 }.into();
                } else {
                    self.r[result] = lhs.sub((right as f64).into(), ctx)?;
                }
            }
            OpCode::SubImmF32 {
                result,
                left,
                right,
            } => {
                let lhs = self.r[left];
                if likely(lhs.is_number()) {
                    self.r[result] = unsafe { lhs.as_number_uncheck() - right as f64 }.into();
                } else {
                    self.r[result] = lhs.sub((right as f64).into(), ctx)?;
                }
            }
            OpCode::Mul {
                result,
                left,
                right,
            } => {
                let lhs = self.r[left];
                let rhs = self.r[right];

                if likely(lhs.is_number() && rhs.is_number()) {
                    unsafe {
                        self.r[result] = JValue::create_number(
                            lhs.as_number_uncheck() * rhs.as_number_uncheck(),
                        );
                    }
                } else {
                    self.r[result] = lhs.mul(rhs, ctx)?;
                };
            }
            OpCode::MulImmI32 {
                result,
                left,
                right,
            } => {
                let lhs = self.r[left];
                if likely(lhs.is_number()) {
                    self.r[result] = unsafe { lhs.as_number_uncheck() * right as f64 }.into();
                } else {
                    self.r[result] = lhs.mul((right as f64).into(), ctx)?;
                }
            }
            OpCode::MulImmF32 {
                result,
                left,
                right,
            } => {
                let lhs = self.r[left];
                if likely(lhs.is_number()) {
                    self.r[result] = unsafe { lhs.as_number_uncheck() * right as f64 }.into();
                } else {
                    self.r[result] = lhs.mul((right as f64).into(), ctx)?;
                }
            }
            OpCode::Div {
                result,
                left,
                right,
            } => {
                let lhs = self.r[left];
                let rhs = self.r[right];

                if likely(lhs.is_number() && rhs.is_number()) {
                    self.r[result] = unsafe {
                        JValue::create_number(lhs.as_number_uncheck() / rhs.as_number_uncheck())
                    };
                } else {
                    self.r[result] = lhs.div(rhs, ctx)?;
                };
            }
            OpCode::DivImmI32 {
                result,
                left,
                right,
            } => {
                let lhs = self.r[left];
                if likely(lhs.is_number()) {
                    self.r[result] = unsafe { lhs.as_number_uncheck() / right as f64 }.into();
                } else {
                    self.r[result] = lhs.div((right as f64).into(), ctx)?;
                }
            }
            OpCode::DivImmF32 {
                result,
                left,
                right,
            } => {
                let lhs = self.r[left];
                if likely(lhs.is_number()) {
                    self.r[result] = unsafe { lhs.as_number_uncheck() / right as f64 }.into();
                } else {
                    self.r[result] = lhs.div((right as f64).into(), ctx)?;
                }
            }
            OpCode::Rem {
                result,
                left,
                right,
            } => {
                let lhs = self.r[left];
                let rhs = self.r[right];

                if likely(lhs.is_number() && rhs.is_number()) {
                    self.r[result] = unsafe {
                        JValue::create_number(lhs.as_number_uncheck() % rhs.as_number_uncheck())
                    };
                } else {
                    self.r[result] = lhs.rem(rhs, ctx)?;
                };
            }
            OpCode::RemImmI32 {
                result,
                left,
                right,
            } => {
                let lhs = self.r[left];
                if likely(lhs.is_number()) {
                    self.r[result] = unsafe { lhs.as_number_uncheck() % right as f64 }.into();
                } else {
                    self.r[result] = lhs.rem((right as f64).into(), ctx)?;
                }
            }
            OpCode::RemImmF32 {
                result,
                left,
                right,
            } => {
                let lhs = self.r[left];
                if likely(lhs.is_number()) {
                    self.r[result] = unsafe { lhs.as_number_uncheck() % right as f64 }.into();
                } else {
                    self.r[result] = lhs.rem((right as f64).into(), ctx)?;
                }
            }
            OpCode::And {
                result,
                left,
                right,
            } => {
                self.r[result] = (self.r[left].to_bool() && self.r[right].to_bool()).into();
            }
            OpCode::AndImm {
                result,
                left,
                right,
            } => {
                let lhs = self.r[left];
                self.r[result] = (lhs.to_bool() && right).into();
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
                };
            }

            OpCode::Not { result, right } => {
                self.r[result] = (!self.r[right].to_bool()).into();
            }

            OpCode::Exp {
                result,
                left,
                right,
            } => {
                let lhs = self.r[left];
                let rhs = self.r[right];

                if likely(lhs.is_number() && rhs.is_number()) {
                    unsafe {
                        self.r[result] = JValue::create_number(
                            lhs.as_number_uncheck().powf(rhs.as_number_uncheck()),
                        );
                    }
                } else {
                    self.r[result] = lhs.exp(rhs, ctx)?;
                };
            }
            OpCode::ExpImmI32 {
                result,
                left,
                right,
            } => {
                let lhs = self.r[left];
                if likely(lhs.is_number()) {
                    self.r[result] = unsafe { lhs.as_number_uncheck().powf(right as f64) }.into();
                } else {
                    self.r[result] = lhs.exp((right as f64).into(), ctx)?;
                }
            }
            OpCode::ExpImmF32 {
                result,
                left,
                right,
            } => {
                let lhs = self.r[left];
                if likely(lhs.is_number()) {
                    self.r[result] = unsafe { lhs.as_number_uncheck().powf(right as f64) }.into();
                } else {
                    self.r[result] = lhs.exp((right as f64).into(), ctx)?;
                }
            }

            OpCode::Plus { result, right } => {
                let rhs = self.r[right];

                if likely(rhs.is_number()) {
                    self.r[result] = rhs;
                } else if likely(rhs.is_int()) {
                    self.r[result] = rhs;
                } else {
                    self.r[result] = self.r[right].to_number(ctx)?.into();
                }
            }
            OpCode::Minus { result, right } => {
                let v = self.r[right];

                if likely(v.is_number()) {
                    self.r[result] = unsafe { JValue::create_number(-v.as_number_uncheck()) };
                } else if likely(v.is_int()) {
                    self.r[result] = JValue::create_int(-v.as_int_unchecked());
                } else {
                    self.r[result] = (-v.to_number(ctx)?).into();
                };
            }

            OpCode::LShift {
                result,
                left,
                right,
            } => {
                let lhs = self.r[left];
                let rhs = self.r[right];

                if likely(lhs.is_number() && rhs.is_number()) {
                    self.r[result] = unsafe {
                        JValue::create_number(
                            ((lhs.as_number_uncheck() as i32) << (rhs.as_number_uncheck() as i32))
                                as f64,
                        )
                    }
                } else {
                    self.r[result] = lhs.lshift(rhs, ctx)?;
                };
            }
            OpCode::LShiftImmI32 {
                result,
                left,
                right,
            } => {
                let lhs = self.r[left];
                if likely(lhs.is_number()) {
                    self.r[result] =
                        unsafe { (lhs.as_number_uncheck() as i32) << (right as i32) }.into();
                } else {
                    self.r[result] = ((lhs.to_i32(ctx)? as i32) << (right as i32)).into();
                }
            }
            OpCode::RShift {
                result,
                left,
                right,
            } => {
                let lhs = self.r[left];
                let rhs = self.r[right];

                if likely(lhs.is_number() && rhs.is_number()) {
                    self.r[result] = unsafe {
                        JValue::create_number(
                            ((lhs.as_number_uncheck() as i32) >> (rhs.as_number_uncheck() as i32))
                                as f64,
                        )
                    }
                } else {
                    self.r[result] = lhs.rshift(rhs, ctx)?;
                };
            }
            OpCode::RShiftImmI32 {
                result,
                left,
                right,
            } => {
                let lhs = self.r[left];
                if likely(lhs.is_number()) {
                    self.r[result] =
                        unsafe { (lhs.as_number_uncheck() as i32) >> (right as i32) }.into();
                } else {
                    self.r[result] = ((lhs.to_i32(ctx)?) >> (right as i32)).into();
                }
            }
            OpCode::ZeroFillRShift {
                result,
                left,
                right,
            } => {
                let lhs = self.r[left];
                let rhs = self.r[right];
                if likely(lhs.is_number() && rhs.is_number()) {
                    self.r[result] = unsafe {
                        JValue::create_number(
                            ((lhs.as_number_uncheck() as u32) >> (rhs.as_number_uncheck() as u32))
                                as f64,
                        )
                    }
                } else {
                    self.r[result] = lhs.unsigned_rshift(rhs, ctx)?;
                };
            }
            OpCode::ZeroFillRShiftImmI32 {
                result,
                left,
                right,
            } => {
                let lhs = self.r[left];
                if likely(lhs.is_number()) {
                    self.r[result] =
                        unsafe { (lhs.as_number_uncheck() as u32) >> (right as u32) }.into();
                } else {
                    self.r[result] = ((lhs.to_i32(ctx)? as u32) >> (right as u32)).into();
                }
            }
            OpCode::Gt {
                result,
                left,
                right,
            } => {
                let lhs = self.r[left];
                let rhs = self.r[right];

                if likely(lhs.is_number() && rhs.is_number()) {
                    self.r[result] =
                        unsafe { lhs.as_number_uncheck() > rhs.as_number_uncheck() }.into();
                } else {
                    self.r[result] = (lhs.to_number(ctx)? > rhs.to_number(ctx)?).into();
                };
            }
            OpCode::GtImmI32 {
                result,
                left,
                right,
            } => {
                let lhs = self.r[left];
                if likely(lhs.is_number()) {
                    self.r[result] = unsafe { (lhs.as_number_uncheck()) > (right as f64) }.into();
                } else {
                    self.r[result] = ((lhs.to_number(ctx)?) > (right as f64)).into();
                }
            }
            OpCode::GtImmF32 {
                result,
                left,
                right,
            } => {
                let lhs = self.r[left];
                if likely(lhs.is_number()) {
                    self.r[result] = unsafe { (lhs.as_number_uncheck()) > (right as f64) }.into();
                } else {
                    self.r[result] = ((lhs.to_number(ctx)?) > (right as f64)).into();
                }
            }
            OpCode::GtEq {
                result,
                left,
                right,
            } => {
                let lhs = self.r[left];
                let rhs = self.r[right];

                if likely(lhs.is_number() && rhs.is_number()) {
                    self.r[result] =
                        unsafe { lhs.as_number_uncheck() >= rhs.as_number_uncheck() }.into();
                } else {
                    self.r[result] = (lhs.to_number(ctx)? >= rhs.to_number(ctx)?).into();
                };
            }
            OpCode::GtEqImmI32 {
                result,
                left,
                right,
            } => {
                let lhs = self.r[left];
                if likely(lhs.is_number()) {
                    self.r[result] = unsafe { (lhs.as_number_uncheck()) >= (right as f64) }.into();
                } else {
                    self.r[result] = ((lhs.to_number(ctx)?) >= (right as f64)).into();
                }
            }
            OpCode::GtEqImmF32 {
                result,
                left,
                right,
            } => {
                let lhs = self.r[left];
                if likely(lhs.is_number()) {
                    self.r[result] = unsafe { (lhs.as_number_uncheck()) >= (right as f64) }.into();
                } else {
                    self.r[result] = ((lhs.to_number(ctx)?) >= (right as f64)).into();
                }
            }
            OpCode::Lt {
                result,
                left,
                right,
            } => {
                let rhs = self.r[right];
                let lhs = self.r[left];

                if likely(rhs.is_number() && lhs.is_number()) {
                    unsafe {
                        self.r[result] = (lhs.as_number_uncheck() < rhs.as_number_uncheck()).into()
                    }
                } else {
                    self.r[result] = (lhs.to_number(ctx)? < rhs.to_number(ctx)?).into();
                }
            }
            OpCode::LtImmI32 {
                result,
                left,
                right,
            } => {
                let lhs = self.r[left];
                if likely(lhs.is_number()) {
                    self.r[result] = unsafe { (lhs.as_number_uncheck()) < (right as f64) }.into();
                } else {
                    self.r[result] = ((lhs.to_number(ctx)?) < (right as f64)).into();
                }
            }
            OpCode::LtImmF32 {
                result,
                left,
                right,
            } => {
                let lhs = self.r[left];
                if likely(lhs.is_number()) {
                    self.r[result] = unsafe { (lhs.as_number_uncheck()) < (right as f64) }.into();
                } else {
                    self.r[result] = ((lhs.to_number(ctx)?) < (right as f64)).into();
                }
            }
            OpCode::LtEq {
                result,
                left,
                right,
            } => {
                let lhs = self.r[left];
                let rhs = self.r[right];

                if likely(lhs.is_number() && rhs.is_number()) {
                    self.r[result] =
                        unsafe { lhs.as_number_uncheck() <= rhs.as_number_uncheck() }.into();
                } else {
                    self.r[result] = (lhs.to_number(ctx)? <= rhs.to_number(ctx)?).into();
                };
            }
            OpCode::LtEqImmI32 {
                result,
                left,
                right,
            } => {
                let lhs = self.r[left];
                if likely(lhs.is_number()) {
                    self.r[result] = unsafe { (lhs.as_number_uncheck()) <= (right as f64) }.into();
                } else {
                    self.r[result] = ((lhs.to_number(ctx)?) <= (right as f64)).into();
                }
            }
            OpCode::LtEqImmF32 {
                result,
                left,
                right,
            } => {
                let lhs = self.r[left];
                if likely(lhs.is_number()) {
                    self.r[result] = unsafe { (lhs.as_number_uncheck()) <= (right as f64) }.into();
                } else {
                    self.r[result] = ((lhs.to_number(ctx)?) <= (right as f64)).into();
                }
            }
            OpCode::EqEq {
                result,
                left,
                right,
            } => {
                let lhs = self.r[left];
                let rhs = self.r[right];

                if likely(lhs.is_number() && rhs.is_number()) {
                    self.r[result] =
                        unsafe { lhs.as_number_uncheck() == rhs.as_number_uncheck() }.into();
                } else {
                    self.r[result] = lhs.eqeq(rhs, ctx)?.into();
                }
            }
            OpCode::EqEqImmI32 {
                result,
                left,
                right,
            } => {
                let lhs = self.r[left];
                if likely(lhs.is_number()) {
                    self.r[result] = unsafe { (lhs.as_number_uncheck()) == (right as f64) }.into();
                } else {
                    self.r[result] = lhs.eqeq((right as f64).into(), ctx)?.into();
                }
            }
            OpCode::EqEqImmF32 {
                result,
                left,
                right,
            } => {
                let lhs = self.r[left];
                if likely(lhs.is_number()) {
                    self.r[result] = unsafe { (lhs.as_number_uncheck()) == (right as f64) }.into();
                } else {
                    self.r[result] = lhs.eqeq((right as f64).into(), ctx)?.into();
                }
            }
            OpCode::EqEqEq {
                result,
                left,
                right,
            } => {
                let lhs = self.r[left];
                let rhs = self.r[right];

                if lhs.to_bits() == rhs.to_bits() {
                    self.r[result] = true.into();
                } else {
                    self.r[result] = (lhs == rhs).into();
                }
            }
            OpCode::EqEqEqImmI32 {
                result,
                left,
                right,
            } => {
                let lhs = self.r[left];
                self.r[result] =
                    unsafe { lhs.is_number() && lhs.as_number_uncheck() == right as f64 }.into();
            }
            OpCode::EqEqEqImmF32 {
                result,
                left,
                right,
            } => {
                let lhs = self.r[left];
                self.r[result] =
                    unsafe { lhs.is_number() && lhs.as_number_uncheck() == right as f64 }.into();
            }
            OpCode::NotEq {
                result,
                left,
                right,
            } => {
                let lhs = self.r[left];
                let rhs = self.r[right];

                if likely(lhs.is_number() && rhs.is_number()) {
                    self.r[result] =
                        unsafe { lhs.as_number_uncheck() != rhs.as_number_uncheck() }.into();
                }
                self.r[result] = (!lhs.eqeq(rhs, ctx)?).into();
            }
            OpCode::NotEqImmI32 {
                result,
                left,
                right,
            } => {
                let lhs = self.r[left];
                if likely(lhs.is_number()) {
                    self.r[result] = unsafe { lhs.as_number_uncheck() != right as f64 }.into();
                } else {
                    self.r[result] = (!lhs.eqeq((right as f64).into(), ctx)?).into();
                }
            }
            OpCode::NotEqImmF32 {
                result,
                left,
                right,
            } => {
                let lhs = self.r[left];
                if likely(lhs.is_number()) {
                    self.r[result] = { lhs.as_number_uncheck() != right as f64 }.into();
                } else {
                    self.r[result] = (!lhs.eqeq((right as f64).into(), ctx)?).into();
                }
            }
            OpCode::NotEqEq {
                result,
                left,
                right,
            } => {
                let lhs = self.r[left];
                let rhs = self.r[right];

                self.r[result] = (!(lhs == rhs)).into();
            }
            OpCode::NotEqEqImmI32 {
                result,
                left,
                right,
            } => {
                let lhs = self.r[left];
                self.r[result] =
                    unsafe { !(lhs.is_number() && lhs.as_number_uncheck() == right as f64) }.into();
            }
            OpCode::NotEqEqImmF32 {
                result,
                left,
                right,
            } => {
                let lhs = self.r[left];
                self.r[result] =
                    unsafe { !(lhs.is_number() && lhs.as_number_uncheck() == right as f64) }.into();
            }
            OpCode::BitAnd {
                result,
                left,
                right,
            } => {
                let lhs = self.r[left];
                let rhs = self.r[right];

                if likely(lhs.is_number() && rhs.is_number()) {
                    self.r[result] =
                        unsafe { lhs.as_number_uncheck() as i32 & rhs.as_number_uncheck() as i32 }
                            .into();
                } else {
                    self.r[result] = lhs.bitand(rhs, ctx)?;
                };
            }
            OpCode::BitAndImmI32 {
                result,
                left,
                right,
            } => {
                let lhs = self.r[left];

                if likely(lhs.is_number()) {
                    self.r[result] =
                        (lhs.as_number_uncheck() as i32 & right as i32).into();
                } else {
                    self.r[result] = (lhs.to_i32(ctx)? & right as i32).into();
                };
            }
            OpCode::BitOr {
                result,
                left,
                right,
            } => {
                let lhs = self.r[left];
                let rhs = self.r[right];

                if likely(lhs.is_number() && rhs.is_number()) {
                    self.r[result] =
                        unsafe { lhs.as_number_uncheck() as i32 | rhs.as_number_uncheck() as i32 }
                            .into();
                } else {
                    self.r[result] = lhs.bitor(rhs, ctx)?;
                };
            }
            OpCode::BitOrImmI32 {
                result,
                left,
                right,
            } => {
                let lhs = self.r[left];

                if likely(lhs.is_number()) {
                    self.r[result] =
                        unsafe { lhs.as_number_uncheck() as i32 | right as i32 }.into();
                } else {
                    self.r[result] = (lhs.to_i32(ctx)? | right as i32).into();
                };
            }
            OpCode::BitXor {
                result,
                left,
                right,
            } => {
                let lhs = self.r[left];
                let rhs = self.r[right];

                if likely(lhs.is_number() && rhs.is_number()) {
                    self.r[result] =
                        unsafe { lhs.as_number_uncheck() as i32 ^ rhs.as_number_uncheck() as i32 }
                            .into();
                } else {
                    self.r[result] = lhs.bitxor(rhs, ctx)?;
                };
            }
            OpCode::BitXorImmI32 {
                result,
                left,
                right,
            } => {
                let lhs = self.r[left];

                if likely(lhs.is_number()) {
                    self.r[result] =
                        unsafe { lhs.as_number_uncheck() as i32 ^ right as i32 }.into();
                } else {
                    self.r[result] = (lhs.to_i32(ctx)? ^ right as i32).into();
                };
            }
            OpCode::BitNot { result, right } => {
                let rhs = self.r[right];

                if likely(rhs.is_number()) {
                    self.r[result] = unsafe { !(rhs.as_number_uncheck() as i32) }.into();
                } else {
                    self.r[result] = (!rhs.to_i32(ctx)?).into();
                }
            }

            OpCode::Await { result, future } => return Ok(Res::Await(self.r[future], result)),
            OpCode::Yield { result, arg } => return Ok(Res::Yield(self.r[arg], result)),
            OpCode::In {
                result,
                left,
                right,
            } => {
                let lhs = self.r[left];
                let rhs = self.r[right];

                if let Some(obj) = rhs.as_object() {
                    let key = lhs.to_property_key(ctx)?;
                    self.r[result] = obj.has_property(key).into();
                } else {
                    return Err(Error::TypeError(format!(
                        "Cannot use 'in' operator to search for '{}' in {}",
                        lhs.to_string(),
                        rhs.to_string()
                    ))
                    .into());
                }
            }
            OpCode::PrivateIn {
                result,
                name,
                right,
            } => {
                let rhs = self.r[right];

                let key = self.runtime.get_field_name(name);

                if let Some(obj) = rhs.as_object() {
                    self.r[result] = obj.has_property(PropKey(name)).into();
                } else {
                    return Err(Error::TypeError(format!(
                        "Cannot use 'in' operator to search for '#{}' in {}",
                        key,
                        rhs.to_string()
                    ))
                    .into());
                }
            }
            OpCode::InstanceOf {
                result,
                left,
                right,
            } => {
                self.r[result] = self.r[left].instance_of(self.r[right], ctx)?;
            }
            OpCode::TypeOf { result, right } => {
                let s = self.r[right].typ().as_str();
                self.r[result] = JValue::create_static_string(s);
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

            OpCode::Throw { value } => return Err(self.r[value]),
            OpCode::Return { value } => return Ok(Res::Return(self.r[value])),

            ///////////////////////////////////////////////////////////////////////
            //   iterator
            //////////////////////////////////////////////////////////////////////
            OpCode::PrepareForIn { target } => {
                todo!();
                let obj = self.r[target];
                let iter = JSIterator::new(obj, ctx)?;
                self.iterators.push(iter);
                
            }
            OpCode::PrepareForOf { target } => {
                let obj = self.r[target];
                let iter = JSIterator::new(obj, ctx)?;
                self.iterators.push(iter);

            }
            OpCode::IterDrop => {
                self.iterators.pop();
            }
            OpCode::IterNext {
                result,
                done,
                hint: _,
                stack_offset,
            } => {
                let iter = self.iterators.last_mut().unwrap();
                let re = iter.next();
                match re {
                    Some(v) => match v {
                        Ok(v) => self.r[result] = v,
                        Err(e) => return Err(e),
                    },
                    None => {
                        self.r[result] = JValue::UNDEFINED;
                        self.r[done] = JValue::TRUE;
                    }
                };
            }
            OpCode::IterCollect {
                result,
                stack_offset,
            } => {
                let iter = self.iterators.last_mut().unwrap();

                let mut values = Vec::new();
                for i in iter {
                    match i {
                        Ok(v) => {
                            values.push((PropFlag::THREE, v));
                        }
                        Err(e) => return Err(e),
                    }
                }
                let obj = JObject::with_array(values);
                self.r[result] = obj.into();
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
                self.r[value] = *unsafe { self.temps.get_unchecked(self.temps.len() - 1) };
            }
            OpCode::ReleaseTemp => {
                unsafe { self.temps.set_len(self.temps.len() - 1) };
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

            OpCode::DeclareDynamicVar {
                from,
                kind: _,
                offset,
            } => {
                let v = self
                    .runtime
                    .to_mut()
                    .stack
                    .get_mut(offset as usize)
                    .unwrap();
                *v = self.r[from];
            }
            OpCode::WriteDynamicVar { from, id } => {
                self.runtime.to_mut().set_variable(id, self.r[from]);
            }
            OpCode::ReadDynamicVar { result, id } => {
                match self.runtime.to_mut().get_variable(id) {
                    Err(e) => return Err(e),
                    Ok(v) => {
                        self.r[result] = v;
                    }
                };
            }
            OpCode::WriteDynamicVarDeclared { from, offset } => {
                let v = unsafe {
                    self.runtime
                        .to_mut()
                        .stack
                        .get_unchecked_mut(offset as usize)
                };
                *v = self.r[from];
            }
            OpCode::ReadDynamicVarDeclared { result, offset } => {
                let v = unsafe { self.runtime.stack.get_unchecked(offset as usize) };
                self.r[result] = *v;
            }
            OpCode::ReadCapturedVar { result, offset } => {
                if let Some(c) = &mut self.capture_stack {
                    self.r[result] = c[offset as usize];
                } else {
                    #[cfg(test)]
                    panic!("reading capture variable on global context")
                }
            }
            OpCode::WriteCapturedVar { from, offset } => {
                if let Some(c) = &mut self.capture_stack {
                    c[offset as usize] = self.r[from];
                } else {
                    #[cfg(test)]
                    panic!("capture variable on global context")
                }
            }
            OpCode::SetThis { value } => {
                *this = self.r[value];
            }

            ////////////////////////////////////////////////////////////////
            //            function
            ////////////////////////////////////////////////////////////////
            OpCode::CreateArg {
                stack_offset: _,
                len: _,
            } => {}
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
                self.stack[stack_offset as usize] = self.r[value];
            }
            OpCode::SpreadArg {
                base_stack_offset,
                stack_offset,
                args_len,
            } => {
                todo!()
            }
            OpCode::FinishArgs {
                base_stack_offset: _,
                len,
            } => {
                self.arg_len = (len as i64 + self.arg_offset_counter) as usize;
                self.arg_offset_counter = 0;
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
                        unsafe { obj.as_array().unwrap_unchecked() }.push((Default::default(), *i));
                    }
                }
                self.r[result] = JValue::create_object(obj);
            }

            OpCode::Call {
                result,
                this,
                callee,
                stack_offset,
                args_len,
            } => {
                let callee = self.r[callee];
                let this = self.r[this];
                let args =
                    &mut self.stack[stack_offset as usize..args_len as usize + stack_offset as usize];
                let stack = unsafe { args.as_mut_ptr().add(args_len as usize) };

                let v = callee.call(
                    this,
                    args,
                    JSContext {
                        stack: stack,
                        runtime: self.runtime,
                    },
                )?;

                self.r[result] = v;
            }
            OpCode::New {
                result,
                callee,
                stack_offset,
                args_len
            } => {
                let stack = &mut self.stack[stack_offset as usize..];

                let constructor = self.r[callee];
                let mut r = Default::default();
                operations::invoke_new(
                    constructor,
                    self.runtime,
                    stack.as_mut_ptr(),
                    self.arg_len,
                    &mut r
                );
                let v = r.0;
                let error = r.1;

                if error {
                    return Err(v);
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
            }
            OpCode::SwitchToBlock(b) => {
                //self.insert_block(b, *index)
            }
            OpCode::Jump { to, line } => {
                *index = line as usize;
                return Ok(Res::Ok);
            }
            OpCode::JumpIfFalse { value, to, line } => {
                if !self.r[value].to_bool() {
                    *index = line as usize;
                    return Ok(Res::Ok);
                }
            }
            OpCode::JumpIfTrue { value, to:_, line } => {
                if self.r[value].to_bool() {
                    *index = line as usize;
                    return Ok(Res::Ok);
                }
            }
            OpCode::EnterTry { catch_block, line } => {
                self.catch_block.push((catch_block, line));
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
                self.r[result] = JValue::create_bigint(value as i64);
            }
            OpCode::LoadStaticFloat { result, id } => {
                self.r[result] = self.runtime.get_unamed_constant(id);
            }
            OpCode::LoadStaticFloat32 { result, value } => {
                self.r[result] = JValue::create_number(value as f64);
            }
            OpCode::LoadStaticString { result, id } => {
                let s = self.runtime.get_string(id);
                self.r[result] = JValue::create_string(self.runtime.allocate_string(s));
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

                self.r[result] = obj.get_property(
                    PropKey(id),
                    JSContext {
                        stack: stack.as_mut_ptr(),
                        runtime: self.runtime,
                    },
                )?;
            }
            OpCode::ReadFieldStatic {
                obj,
                result,
                field_id,
            } => {
                let obj = self.r[obj];

                self.r[result] = obj.get_property(PropKey(field_id), ctx)?;
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

                obj.set_property(
                    PropKey(id),
                    self.r[value],
                    JSContext {
                        stack: stack.as_mut_ptr(),
                        runtime: self.runtime,
                    },
                )?;
            }
            OpCode::WriteFieldStatic {
                obj,
                value,
                field_id,
            } => {
                let obj = self.r[obj];
                let value = self.r[value];

                let re = obj.set_property(PropKey(field_id), value, ctx);
                match re {
                    Ok(()) => {}
                    Err(e) => return Err(e),
                }
            }

            OpCode::RemoveFieldStatic { obj, field_id } => {
                let obj = self.r[obj];
                if let Some(obj) = obj.as_object() {
                    obj.remove_property(PropKey(field_id));
                }
            }

            OpCode::ReadSuperField {
                constructor,
                result,
                field,
                stack_offset,
            } => {
                let stack = &mut self.stack[stack_offset as usize..];
                let mut r = Default::default();
                unsafe {
                    operations::super_prop(
                        &self.runtime,
                        self.r[constructor],
                        self.r[field],
                        stack.as_mut_ptr(),
                        &mut r
                    )
                };
                let v = r.0;
                let err = r.1;
                if err {
                    return Err(v);
                }
                self.r[result] = v;
            }
            OpCode::ReadSuperFieldStatic {
                constructor,
                result,
                field_id,
            } => {
                let mut r = Default::default();
                unsafe {
                    operations::super_prop_static(
                        &self.runtime,
                        self.r[constructor],
                        field_id,
                        ctx.stack,
                        &mut r
                    )
                };
                let v = r.0;
                let err = r.1;
                if err {
                    return Err(v);
                }
                self.r[result] = v;
            }
            OpCode::WriteSuperField {
                constructor,
                value,
                field,
            } => {
                let mut r = Default::default();
                unsafe {
                    operations::super_write_prop(
                        &self.runtime,
                        self.r[constructor],
                        self.r[field],
                        self.r[value],
                        ctx.stack,
                        &mut r
                    )
                };
                let v = r.0;
                let err = r.1;
                if err {
                    return Err(v);
                };
            }
            OpCode::WriteSuperFieldStatic {
                constructor,
                value,
                field,
            } => {
                let mut r = Default::default();
                unsafe {
                    operations::super_write_prop_static(
                        &self.runtime,
                        self.r[constructor],
                        field,
                        self.r[value],
                        ctx.stack,
                        &mut r
                    )
                };
                let v = r.0;
                let err = r.1;
                if err {
                    return Err(v);
                };
            }

            OpCode::BindGetter {
                obj,
                field_id,
                getter,
            } => {
                let obj = self.r[obj];
                if let Some(obj) = obj.as_object() {
                    unsafe {
                        obj.bind_getter(PropKey(field_id), self.r[getter].as_object().unwrap())
                    }
                }
            }
            OpCode::BindSetter {
                obj,
                field_id,
                setter,
            } => {
                let obj = self.r[obj];
                if let Some(obj) = obj.as_object() {
                    obj.bind_setter(PropKey(field_id), self.r[setter].as_object().unwrap());
                }
            }
            OpCode::CloneObject { obj, result } => {
                let obj = self.r[obj];
                if let Some(obj) = obj.as_object() {
                    self.r[result] = JValue::create_object(unsafe { obj.deep_clone() });
                }
            }
            OpCode::ExtendObject { obj, from } => {
                unsafe { operations::extend_object(self.r[obj], self.r[from], self.runtime) };
            }

            ////////////////////////////////////////////////////////////////
            //           creations
            ///////////////////////////////////////////////////////////////
            OpCode::CreateArray { result, stack_offset } => {
                let e = &mut self.stack[stack_offset as usize..stack_offset as usize + self.arg_len];
                let e = e.iter().map(|v|(Default::default(), *v)).collect();
                let array = JObject::with_array(e);
                self.r[result] = JValue::create_object(array);
            }
            OpCode::CreateArrow { result, this, id } => {
                let f = if self.is_global {
                    unsafe {
                        self.runtime
                            .get_function(id)
                            .unwrap_unchecked()
                            .create_instance(Some(self.r[this]))
                    }
                } else {
                    unsafe {
                        self.runtime
                            .get_function(id)
                            .unwrap_unchecked()
                            .create_instance_with_capture(
                                Some(self.r[this]),
                                self.cap.clone().unwrap(),
                            )
                    }
                };

                self.r[result] = JValue::create_object(JObject::with_function(f));
            }
            OpCode::CreateFunction { result, id } => {
                let f = unsafe {
                    if self.is_global {
                        self.runtime
                            .get_function(id)
                            .unwrap_unchecked()
                            .create_instance(None)
                    } else {
                        self.runtime
                            .get_function(id)
                            .unwrap()
                            .create_instance_with_capture(None, self.cap.clone().unwrap())
                    }
                };

                self.r[result] = JValue::create_object(JObject::with_function(f));
            }

            OpCode::CreateObject { result } => {
                self.r[result] = JValue::create_object(JObject::new());
            }
            OpCode::CreateRegExp { result, reg_id } => {
                let r = self.runtime.get_regex(reg_id);
                self.r[result] = JObject::with_regex(r).into();
            }
            OpCode::CreateTemplate {
                result,
                id,
                stack_offset,
            } => {
                let stack = &mut self.stack[stack_offset as usize..];

                self.r[result] = unsafe {
                    operations::create_template(
                        id.0,
                        stack.as_mut_ptr(),
                        self.arg_len as u32,
                        false,
                    )
                };
            }

            OpCode::CreateClass { result, class_id } => {
                let class = self.runtime.get_class(class_id);

                let obj = if self.is_global {
                    class.ect_without_capture()
                } else {
                    unsafe { class.ect_with_capture(self.cap.clone().unwrap()) }
                };

                self.r[result] = obj.into();
            }
            OpCode::ClassBindSuper { class, super_ } => unsafe {
                let super_class = self.r[super_];
                let c = self.r[class];
                let proto = c.get_property(NAMES["prototype"], ctx).unwrap_unchecked();
                let super_proto = match super_class.get_property(NAMES["prototype"], ctx) {
                    Ok(v) => v,
                    Err(e) => return Err(e),
                };
                match proto.set_property(NAMES["__proto__"], super_proto, ctx) {
                    _ => {}
                };

                if !super_class.is_object() {
                    return Err(JValue::from(Error::ClassExtendsNonCallable));
                }
                if let Some(obj) = c.as_object() {
                    if let Some(c) = obj.as_class() {
                        c.set_super(super_class.as_object().unwrap());
                    }
                }
            },
        };
        Ok(Res::Ok)
    }
}
