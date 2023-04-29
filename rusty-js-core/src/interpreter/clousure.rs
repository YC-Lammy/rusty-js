use likely_stable::likely;

use crate::bultins::object_property::PropFlag;
use crate::bultins::function::CaptureStack;
use crate::bultins::object::JObject;
use crate::bytecodes::{Block, OpCode, Register};
use crate::error::Error;
use crate::runtime::Runtime;
use crate::value::JValue;
use crate::utils::iterator::JSIterator;
use crate::utils::string_interner::NAMES;
use crate::{operations, JSContext, PropKey, Promise};

use super::Registers;
use super::Res;

type F = Box<
    dyn Fn(
        &mut ClousureState,
        JSContext,
        &mut Registers,
        &mut JValue,
        &[JValue],
        &mut [JValue],
        &mut usize,
    ) -> Result<Res, JValue>,
    >;

struct ClousureState<'a> {
    runtime: &'a Runtime,
    op_stack: *mut JValue,

    accumulator: JValue,

    arg_offset_counter: i64,
    arg_len: usize,

    cap: Option<CaptureStack>,
    capture_stack: Option<&'a mut [JValue]>,

    /// in a try statement
    catch_block: Vec<(Block, u32)>,

    is_global: bool,

    iterators: Vec<JSIterator<'a>>,

    temps: Vec<JValue>,
}

pub struct Clousure {
    codes: Vec<
        Box<
            dyn Fn(
                &mut ClousureState,
                JSContext,
                &mut Registers,
                &mut JValue,
                &[JValue],
                &mut [JValue],
                &mut usize,
            ) -> Result<Res, JValue>,
        >,
    >,
}

#[allow(unused)]
impl Clousure {
    pub fn create(codes: &[OpCode]) -> Self {
        let f = codes
            .iter()
            .map(|c| Self::create_code(*c))
            .collect::<Vec<_>>();
        Self { codes: f }
    }

    pub fn run<'a>(
        &mut self,
        runtime: &Runtime,
        stack: &mut [JValue],
        op_stack: &mut [JValue],
        capture_stack: Option<CaptureStack>,
        capture_stack_data: Option<&mut [JValue]>,
        mut this: JValue,
        args: &[JValue],
    ) -> Result<JValue, JValue> {
        let mut i = 0;

        let is_global = capture_stack_data.is_none();

        let mut state = ClousureState {
            runtime,
            op_stack: op_stack.as_mut_ptr(),
            accumulator: JValue::UNDEFINED,

            arg_offset_counter: 0,
            arg_len: 0,
            cap: capture_stack,
            capture_stack: capture_stack_data,
            catch_block: Default::default(),
            is_global: is_global,
            iterators: Vec::new(),
            temps: Default::default(),
        };

        let mut regs = Registers([JValue::UNDEFINED; 3]);
        let ctx = JSContext {
            stack: state.op_stack,
            runtime: state.runtime,
        };

        loop {
            if i == self.codes.len() {
                break;
            }
            let code = unsafe{self.codes.get_unchecked_mut(i)};

            //debug::debug!("run code {:#?}", code);

            let re = (code)(&mut state, ctx, &mut regs, &mut this, args, stack, &mut i);

            match re {
                Err(e) => {
                    if let Some((_catch_block, line)) = state.catch_block.pop() {
                        regs[Register(0)] = e;

                        i = line as usize;
                    } else {
                        return Err(e);
                    }
                }
                Ok(re) => match re {
                    Res::Ok => {}
                    Res::Return(r) => return Ok(r),
                    _ => {}
                },
            }

            i += 1;
        }
        Ok(JValue::UNDEFINED)
    }

    async fn run_async<'a>(
        &mut self,
        runtime: &Runtime,
        stack: &mut [JValue],
        op_stack: &mut [JValue],
        capture_stack: Option<CaptureStack>,
        capture_stack_data: Option<&mut [JValue]>,
        mut this: JValue,
        args: &[JValue],
        mut yield_value: tokio::sync::mpsc::Receiver<JValue>,
        mut yielder_sender: tokio::sync::mpsc::Sender<JValue>
    ) -> Result<JValue, JValue>{
        let mut i = 0;

        let is_global = capture_stack_data.is_none();

        let mut state = ClousureState {
            runtime,
            op_stack: op_stack.as_mut_ptr(),
            accumulator: JValue::UNDEFINED,

            arg_offset_counter: 0,
            arg_len: 0,
            cap: capture_stack,
            capture_stack: capture_stack_data,
            catch_block: Default::default(),
            is_global: is_global,
            iterators: Vec::new(),
            temps: Default::default(),
        };

        let mut regs = Registers([JValue::UNDEFINED; 3]);
        let ctx = JSContext {
            stack: state.op_stack,
            runtime: state.runtime,
        };

        loop {
            if i == self.codes.len() {
                break;
            }
            let code = unsafe{self.codes.get_unchecked_mut(i)};

            //debug::debug!("run code {:#?}", code);

            let re = (code)(&mut state, ctx, &mut regs, &mut this, args, stack, &mut i);

            match re {
                Err(e) => {
                    if let Some((_catch_block, line)) = state.catch_block.pop() {
                        regs[Register(0)] = e;

                        i = line as usize;
                    } else {
                        return Err(e);
                    }
                }
                Ok(re) => match re {
                    Res::Ok => {}
                    Res::Return(r) => return Ok(r),
                    Res::Yield(v, r) => {
                        yielder_sender.send(v).await;
                        let v = yield_value.recv().await.unwrap_or(JValue::UNDEFINED);
                        regs[r] = v;
                    },
                    Res::Await(v, r) => {
                        if let Some(o) = v.as_object(){
                            if let Some(p) = o.as_promise(){
                                match p{
                                    Promise::ForeverPending => {
                                        return Err(Error::AwaitOnForeverPendingPromise.into());
                                    },
                                    Promise::Fulfilled(f) => {
                                        regs[r] = *f;
                                    },
                                    Promise::Rejected(e) => {
                                        if let Some((_catch_block, line)) = state.catch_block.pop() {
                                            regs[Register(0)] = *e;
                    
                                            i = line as usize;
                                        } else {
                                            return Err(*e);
                                        }
                                    }
                                    Promise::Pending { id } => {
                                        let re = runtime.to_mut().get_future(*id).await;
                                        match re{
                                            Ok(v) => {
                                                regs[r] = v
                                            }
                                            Err(e) => {
                                                if let Some((_catch_block, line)) = state.catch_block.pop() {
                                                    regs[Register(0)] = e;
                            
                                                    i = line as usize;
                                                } else {
                                                    return Err(e);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        } else{
                            regs[r] = v;
                        }
                    }
                    _ => {}
                },
            }

            i += 1;
        }
        Ok(JValue::UNDEFINED)
    }

    fn create_code<'a>(
        code: OpCode,
    ) -> Box<
        dyn Fn(
            &mut ClousureState,
            JSContext,
            &mut Registers,
            &mut JValue,
            &[JValue],
            &mut [JValue],
            &mut usize,
        ) -> Result<Res, JValue>,
    > {
        //println!("{:?}", code);
        match code {
            OpCode::NoOp => Box::new(
                move |_state: &mut ClousureState,
                      _ctx: JSContext,
                      _regs: &mut Registers,
                      _this: &mut JValue,
                      _args: &[JValue],
                      _stack: &mut [JValue],
                      _index: &mut usize| { Ok(Res::Ok) },
            ),
            OpCode::Debugger => Box::new(
                move |_state: &mut ClousureState,
                      _ctx: JSContext,
                      _regs: &mut Registers,
                      _this: &mut JValue,
                      _args: &[JValue],
                      _stack: &mut [JValue],
                      _index: &mut usize| { Ok(Res::Ok) },
            ),
            OpCode::IsNullish { result, value } => Box::new(
                move |_state: &mut ClousureState,
                ctx: JSContext,
                regs: &mut Registers,
                _this: &mut JValue,
                _args: &[JValue],
                _stack: &mut [JValue],
                _index: &mut usize|{
                    let value = regs[value];
                    regs[result] = (value.is_null() || value.is_undefined()).into();
                    Ok(Res::Ok)
                }
            ),
            OpCode::Add {
                result,
                left,
                right,
            } => Box::new(
                move |_state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      _this: &mut JValue,
                      _args: &[JValue],
                      _stack: &mut [JValue],
                      _index: &mut usize| {
                    let lhs = regs[left];
                    let rhs = regs[right];

                    if likely(lhs.is_number() && rhs.is_number()) {
                        regs[result] = JValue::create_number(
                            lhs.as_number_uncheck() + rhs.as_number_uncheck(),
                        );
                    } else {
                        regs[result] = lhs.add(rhs, ctx)?;
                    };
                    Ok(Res::Ok)
                },
            ),
            OpCode::AddImmI32 {
                result,
                left,
                right,
            } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    let lhs = regs[left];
                    if likely(lhs.is_number()) {
                        regs[result] = (lhs.as_number_uncheck() + right as f64).into();
                    } else if likely(lhs.is_int()) {
                        regs[result] = (lhs.as_int_unchecked() + right as i32).into();
                    } else {
                        regs[result] = (lhs.add((right as f64).into(), ctx))?;
                    };
                    Ok(Res::Ok)
                },
            ),
            OpCode::AddImmF32 {
                result,
                left,
                right,
            } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    let lhs = regs[left];
                    if likely(lhs.is_number()) {
                        regs[result] = unsafe { lhs.as_number_uncheck() + right as f64 }.into();
                    } else {
                        regs[result] = (lhs.add((right as f64).into(), ctx))?;
                    };
                    Ok(Res::Ok)
                },
            ),
            OpCode::AddImmStr { result, left, str } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    let lhs = regs[left];
                    regs[result] = (lhs.to_string() + state.runtime.get_string(str)).into();
                    Ok(Res::Ok)
                },
            ),
            OpCode::Sub {
                result,
                left,
                right,
            } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    let lhs = regs[left];
                    let rhs = regs[right];

                    if likely(lhs.is_number() && rhs.is_number()) {
                        regs[result] = unsafe {
                            JValue::create_number(lhs.as_number_uncheck() - rhs.as_number_uncheck())
                        };
                    } else {
                        regs[result] = lhs.sub(rhs, ctx)?;
                    };
                    Ok(Res::Ok)
                },
            ),
            OpCode::SubImmI32 {
                result,
                left,
                right,
            } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    let lhs = regs[left];
                    if likely(lhs.is_number()) {
                        regs[result] = unsafe { lhs.as_number_uncheck() - right as f64 }.into();
                    } else {
                        regs[result] = lhs.sub((right as f64).into(), ctx)?;
                    };
                    Ok(Res::Ok)
                },
            ),
            OpCode::SubImmF32 {
                result,
                left,
                right,
            } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    let lhs = regs[left];
                    if likely(lhs.is_number()) {
                        regs[result] = unsafe { lhs.as_number_uncheck() - right as f64 }.into();
                    } else {
                        regs[result] = lhs.sub((right as f64).into(), ctx)?;
                    };
                    Ok(Res::Ok)
                },
            ),
            OpCode::Mul {
                result,
                left,
                right,
            } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    let lhs = regs[left];
                    let rhs = regs[right];

                    if likely(lhs.is_number() && rhs.is_number()) {
                        unsafe {
                            regs[result] = JValue::create_number(
                                lhs.as_number_uncheck() * rhs.as_number_uncheck(),
                            );
                        }
                    } else {
                        regs[result] = lhs.mul(rhs, ctx)?;
                    };
                    Ok(Res::Ok)
                },
            ),
            OpCode::MulImmI32 {
                result,
                left,
                right,
            } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    let lhs = regs[left];
                    if likely(lhs.is_number()) {
                        regs[result] = unsafe { lhs.as_number_uncheck() * right as f64 }.into();
                    } else {
                        regs[result] = lhs.mul((right as f64).into(), ctx)?;
                    };
                    Ok(Res::Ok)
                },
            ),
            OpCode::MulImmF32 {
                result,
                left,
                right,
            } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    let lhs = regs[left];
                    if likely(lhs.is_number()) {
                        regs[result] = unsafe { lhs.as_number_uncheck() * right as f64 }.into();
                    } else {
                        regs[result] = lhs.mul((right as f64).into(), ctx)?;
                    };
                    Ok(Res::Ok)
                },
            ),
            OpCode::Div {
                result,
                left,
                right,
            } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    let lhs = regs[left];
                    let rhs = regs[right];

                    if likely(lhs.is_number() && rhs.is_number()) {
                        regs[result] = unsafe {
                            JValue::create_number(lhs.as_number_uncheck() / rhs.as_number_uncheck())
                        };
                    } else {
                        regs[result] = lhs.div(rhs, ctx)?;
                    };
                    Ok(Res::Ok)
                },
            ),
            OpCode::DivImmI32 {
                result,
                left,
                right,
            } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    let lhs = regs[left];
                    if likely(lhs.is_number()) {
                        regs[result] = unsafe { lhs.as_number_uncheck() / right as f64 }.into();
                    } else {
                        regs[result] = lhs.div((right as f64).into(), ctx)?;
                    };
                    Ok(Res::Ok)
                },
            ),
            OpCode::DivImmF32 {
                result,
                left,
                right,
            } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    let lhs = regs[left];
                    if likely(lhs.is_number()) {
                        regs[result] = unsafe { lhs.as_number_uncheck() / right as f64 }.into();
                    } else {
                        regs[result] = lhs.div((right as f64).into(), ctx)?;
                    };
                    Ok(Res::Ok)
                },
            ),
            OpCode::Rem {
                result,
                left,
                right,
            } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    let lhs = regs[left];
                    let rhs = regs[right];

                    if likely(lhs.is_number() && rhs.is_number()) {
                        regs[result] = unsafe {
                            JValue::create_number(lhs.as_number_uncheck() % rhs.as_number_uncheck())
                        };
                    } else {
                        regs[result] = lhs.rem(rhs, ctx)?;
                    };
                    Ok(Res::Ok)
                },
            ),
            OpCode::RemImmI32 {
                result,
                left,
                right,
            } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    let lhs = regs[left];
                    if likely(lhs.is_number()) {
                        regs[result] = unsafe { lhs.as_number_uncheck() % right as f64 }.into();
                    } else {
                        regs[result] = lhs.rem((right as f64).into(), ctx)?;
                    };
                    Ok(Res::Ok)
                },
            ),
            OpCode::RemImmF32 {
                result,
                left,
                right,
            } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    let lhs = regs[left];
                    if likely(lhs.is_number()) {
                        regs[result] = unsafe { lhs.as_number_uncheck() % right as f64 }.into();
                    } else {
                        regs[result] = lhs.rem((right as f64).into(), ctx)?;
                    };
                    Ok(Res::Ok)
                },
            ),
            OpCode::And {
                result,
                left,
                right,
            } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    regs[result] = (regs[left].to_bool() && regs[right].to_bool()).into();
                    Ok(Res::Ok)
                },
            ),
            OpCode::AndImm {
                result,
                left,
                right,
            } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    let lhs = regs[left];
                    regs[result] = (lhs.to_bool() && right).into();
                    Ok(Res::Ok)
                },
            ),
            OpCode::Or {
                result,
                left,
                right,
            } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    if regs[left].to_bool() {
                        regs[result] = regs[left];
                    } else {
                        regs[result] = regs[right];
                    };
                    Ok(Res::Ok)
                },
            ),
            OpCode::Not { result, right } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    regs[result] = (!regs[right].to_bool()).into();
                    Ok(Res::Ok)
                },
            ),

            OpCode::Exp {
                result,
                left,
                right,
            } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    let lhs = regs[left];
                    let rhs = regs[right];

                    if likely(lhs.is_number() && rhs.is_number()) {
                        unsafe {
                            regs[result] = JValue::create_number(
                                lhs.as_number_uncheck().powf(rhs.as_number_uncheck()),
                            );
                        }
                    } else {
                        regs[result] = lhs.exp(rhs, ctx)?;
                    };
                    Ok(Res::Ok)
                },
            ),
            OpCode::ExpImmI32 {
                result,
                left,
                right,
            } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    let lhs = regs[left];
                    if likely(lhs.is_number()) {
                        regs[result] = unsafe { lhs.as_number_uncheck().powf(right as f64) }.into();
                    } else {
                        regs[result] = lhs.exp((right as f64).into(), ctx)?;
                    };
                    Ok(Res::Ok)
                },
            ),
            OpCode::ExpImmF32 {
                result,
                left,
                right,
            } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    let lhs = regs[left];
                    if likely(lhs.is_number()) {
                        regs[result] = unsafe { lhs.as_number_uncheck().powf(right as f64) }.into();
                    } else {
                        regs[result] = lhs.exp((right as f64).into(), ctx)?;
                    };
                    Ok(Res::Ok)
                },
            ),
            OpCode::Plus { result, right } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    let rhs = regs[right];

                    if likely(rhs.is_number()) {
                        regs[result] = rhs;
                    } else if likely(rhs.is_int()) {
                        regs[result] = rhs;
                    } else {
                        regs[result] = regs[right].to_number(ctx)?.into();
                    };
                    Ok(Res::Ok)
                },
            ),
            OpCode::Minus { result, right } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    let v = regs[right];

                    if likely(v.is_number()) {
                        regs[result] = unsafe { JValue::create_number(-v.as_number_uncheck()) };
                    } else if likely(v.is_int()) {
                        regs[result] = JValue::create_int(-v.as_int_unchecked());
                    } else {
                        regs[result] = (-v.to_number(ctx)?).into();
                    };
                    Ok(Res::Ok)
                },
            ),
            OpCode::LShift {
                result,
                left,
                right,
            } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    let lhs = regs[left];
                    let rhs = regs[right];

                    if likely(lhs.is_number() && rhs.is_number()) {
                        regs[result] = unsafe {
                            JValue::create_number(
                                ((lhs.as_number_uncheck() as i32)
                                    << (rhs.as_number_uncheck() as i32))
                                    as f64,
                            )
                        }
                    } else {
                        regs[result] = lhs.lshift(rhs, ctx)?;
                    };
                    Ok(Res::Ok)
                },
            ),
            OpCode::LShiftImmI32 {
                result,
                left,
                right,
            } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    let lhs = regs[left];
                    if likely(lhs.is_number()) {
                        regs[result] =
                            unsafe { (lhs.as_number_uncheck() as i32) << (right as i32) }.into();
                    } else {
                        regs[result] = ((lhs.to_i32(ctx)? as i32) << (right as i32)).into();
                    };
                    Ok(Res::Ok)
                },
            ),
            OpCode::RShift {
                result,
                left,
                right,
            } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    let lhs = regs[left];
                    let rhs = regs[right];

                    if likely(lhs.is_number() && rhs.is_number()) {
                        regs[result] = unsafe {
                            JValue::create_number(
                                ((lhs.as_number_uncheck() as i32)
                                    >> (rhs.as_number_uncheck() as i32))
                                    as f64,
                            )
                        }
                    } else {
                        regs[result] = lhs.rshift(rhs, ctx)?;
                    };
                    Ok(Res::Ok)
                },
            ),
            OpCode::RShiftImmI32 {
                result,
                left,
                right,
            } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    let lhs = regs[left];
                    if likely(lhs.is_number()) {
                        regs[result] =
                            unsafe { (lhs.as_number_uncheck() as i32) >> (right as i32) }.into();
                    } else {
                        regs[result] = ((lhs.to_i32(ctx)?) >> (right as i32)).into();
                    };
                    Ok(Res::Ok)
                },
            ),
            OpCode::ZeroFillRShift {
                result,
                left,
                right,
            } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    let lhs = regs[left];
                    let rhs = regs[right];
                    if likely(lhs.is_number() && rhs.is_number()) {
                        regs[result] = unsafe {
                            JValue::create_number(
                                ((lhs.as_number_uncheck() as u32)
                                    >> (rhs.as_number_uncheck() as u32))
                                    as f64,
                            )
                        }
                    } else {
                        regs[result] = lhs.unsigned_rshift(rhs, ctx)?;
                    };
                    Ok(Res::Ok)
                },
            ),
            OpCode::ZeroFillRShiftImmI32 {
                result,
                left,
                right,
            } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    let lhs = regs[left];
                    if likely(lhs.is_number()) {
                        regs[result] =
                            unsafe { (lhs.as_number_uncheck() as u32) >> (right as u32) }.into();
                    } else {
                        regs[result] = ((lhs.to_i32(ctx)? as u32) >> (right as u32)).into();
                    };
                    Ok(Res::Ok)
                },
            ),
            OpCode::Gt {
                result,
                left,
                right,
            } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    let lhs = regs[left];
                    let rhs = regs[right];

                    if likely(lhs.is_number() && rhs.is_number()) {
                        regs[result] =
                            unsafe { lhs.as_number_uncheck() > rhs.as_number_uncheck() }.into();
                    } else {
                        regs[result] = (lhs.to_number(ctx)? > rhs.to_number(ctx)?).into();
                    };
                    Ok(Res::Ok)
                },
            ),
            OpCode::GtImmI32 {
                result,
                left,
                right,
            } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    let lhs = regs[left];
                    if likely(lhs.is_number()) {
                        regs[result] = unsafe { (lhs.as_number_uncheck()) > (right as f64) }.into();
                    } else {
                        regs[result] = ((lhs.to_number(ctx)?) > (right as f64)).into();
                    };
                    Ok(Res::Ok)
                },
            ),
            OpCode::GtImmF32 {
                result,
                left,
                right,
            } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    let lhs = regs[left];
                    if likely(lhs.is_number()) {
                        regs[result] = unsafe { (lhs.as_number_uncheck()) > (right as f64) }.into();
                    } else {
                        regs[result] = ((lhs.to_number(ctx)?) > (right as f64)).into();
                    };
                    Ok(Res::Ok)
                },
            ),
            OpCode::GtEq {
                result,
                left,
                right,
            } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    let lhs = regs[left];
                    let rhs = regs[right];

                    if likely(lhs.is_number() && rhs.is_number()) {
                        regs[result] =
                            unsafe { lhs.as_number_uncheck() >= rhs.as_number_uncheck() }.into();
                    } else {
                        regs[result] = (lhs.to_number(ctx)? >= rhs.to_number(ctx)?).into();
                    };
                    Ok(Res::Ok)
                },
            ),
            OpCode::GtEqImmI32 {
                result,
                left,
                right,
            } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    let lhs = regs[left];
                    if likely(lhs.is_number()) {
                        regs[result] =
                            unsafe { (lhs.as_number_uncheck()) >= (right as f64) }.into();
                    } else {
                        regs[result] = ((lhs.to_number(ctx)?) >= (right as f64)).into();
                    };
                    Ok(Res::Ok)
                },
            ),
            OpCode::GtEqImmF32 {
                result,
                left,
                right,
            } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    let lhs = regs[left];
                    if likely(lhs.is_number()) {
                        regs[result] =
                            unsafe { (lhs.as_number_uncheck()) >= (right as f64) }.into();
                    } else {
                        regs[result] = ((lhs.to_number(ctx)?) >= (right as f64)).into();
                    };
                    Ok(Res::Ok)
                },
            ),
            OpCode::Lt {
                result,
                left,
                right,
            } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    let rhs = regs[right];
                    let lhs = regs[left];

                    if likely(rhs.is_number() && lhs.is_number()) {
                        unsafe {
                            regs[result] =
                                (lhs.as_number_uncheck() < rhs.as_number_uncheck()).into()
                        }
                    } else {
                        regs[result] = (lhs.to_number(ctx)? < rhs.to_number(ctx)?).into();
                    };
                    Ok(Res::Ok)
                },
            ),
            OpCode::LtImmI32 {
                result,
                left,
                right,
            } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    let lhs = regs[left];
                    if likely(lhs.is_number()) {
                        regs[result] = unsafe { (lhs.as_number_uncheck()) < (right as f64) }.into();
                    } else {
                        regs[result] = ((lhs.to_number(ctx)?) < (right as f64)).into();
                    };
                    Ok(Res::Ok)
                },
            ),
            OpCode::LtImmF32 {
                result,
                left,
                right,
            } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    let lhs = regs[left];
                    if likely(lhs.is_number()) {
                        regs[result] = unsafe { (lhs.as_number_uncheck()) < (right as f64) }.into();
                    } else {
                        regs[result] = ((lhs.to_number(ctx)?) < (right as f64)).into();
                    };
                    Ok(Res::Ok)
                },
            ),
            OpCode::LtEq {
                result,
                left,
                right,
            } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    let lhs = regs[left];
                    let rhs = regs[right];

                    if likely(lhs.is_number() && rhs.is_number()) {
                        regs[result] =
                            unsafe { lhs.as_number_uncheck() <= rhs.as_number_uncheck() }.into();
                    } else {
                        regs[result] = (lhs.to_number(ctx)? <= rhs.to_number(ctx)?).into();
                    };
                    Ok(Res::Ok)
                },
            ),
            OpCode::LtEqImmI32 {
                result,
                left,
                right,
            } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    let lhs = regs[left];
                    if likely(lhs.is_number()) {
                        regs[result] =
                            unsafe { (lhs.as_number_uncheck()) <= (right as f64) }.into();
                    } else {
                        regs[result] = ((lhs.to_number(ctx)?) <= (right as f64)).into();
                    };
                    Ok(Res::Ok)
                },
            ),
            OpCode::LtEqImmF32 {
                result,
                left,
                right,
            } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    let lhs = regs[left];
                    if likely(lhs.is_number()) {
                        regs[result] =
                            unsafe { (lhs.as_number_uncheck()) <= (right as f64) }.into();
                    } else {
                        regs[result] = ((lhs.to_number(ctx)?) <= (right as f64)).into();
                    };
                    Ok(Res::Ok)
                },
            ),
            OpCode::EqEq {
                result,
                left,
                right,
            } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    let lhs = regs[left];
                    let rhs = regs[right];

                    if likely(lhs.is_number() && rhs.is_number()) {
                        regs[result] =
                            unsafe { lhs.as_number_uncheck() == rhs.as_number_uncheck() }.into();
                    } else {
                        regs[result] = lhs.eqeq(rhs, ctx)?.into();
                    };
                    Ok(Res::Ok)
                },
            ),
            OpCode::EqEqImmI32 {
                result,
                left,
                right,
            } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    let lhs = regs[left];
                    if likely(lhs.is_number()) {
                        regs[result] =
                            unsafe { (lhs.as_number_uncheck()) == (right as f64) }.into();
                    } else {
                        regs[result] = lhs.eqeq((right as f64).into(), ctx)?.into();
                    };
                    Ok(Res::Ok)
                },
            ),
            OpCode::EqEqImmF32 {
                result,
                left,
                right,
            } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    let lhs = regs[left];
                    if likely(lhs.is_number()) {
                        regs[result] =
                            unsafe { (lhs.as_number_uncheck()) == (right as f64) }.into();
                    } else {
                        regs[result] = lhs.eqeq((right as f64).into(), ctx)?.into();
                    };
                    Ok(Res::Ok)
                },
            ),
            OpCode::EqEqEq {
                result,
                left,
                right,
            } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    let lhs = regs[left];
                    let rhs = regs[right];

                    if lhs.to_bits() == rhs.to_bits() {
                        regs[result] = true.into();
                    } else {
                        regs[result] = (lhs == rhs).into();
                    };
                    Ok(Res::Ok)
                },
            ),
            OpCode::EqEqEqImmI32 {
                result,
                left,
                right,
            } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    let lhs = regs[left];
                    regs[result] =
                        unsafe { lhs.is_number() && lhs.as_number_uncheck() == right as f64 }
                            .into();
                    Ok(Res::Ok)
                },
            ),
            OpCode::EqEqEqImmF32 {
                result,
                left,
                right,
            } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    let lhs = regs[left];
                    regs[result] =
                        unsafe { lhs.is_number() && lhs.as_number_uncheck() == right as f64 }
                            .into();
                    Ok(Res::Ok)
                },
            ),
            OpCode::NotEq {
                result,
                left,
                right,
            } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    let lhs = regs[left];
                    let rhs = regs[right];

                    if likely(lhs.is_number() && rhs.is_number()) {
                        regs[result] =
                            unsafe { lhs.as_number_uncheck() != rhs.as_number_uncheck() }.into();
                    }
                    regs[result] = (!lhs.eqeq(rhs, ctx)?).into();
                    Ok(Res::Ok)
                },
            ),
            OpCode::NotEqImmI32 {
                result,
                left,
                right,
            } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    let lhs = regs[left];
                    if likely(lhs.is_number()) {
                        regs[result] = unsafe { lhs.as_number_uncheck() != right as f64 }.into();
                    } else {
                        regs[result] = (!lhs.eqeq((right as f64).into(), ctx)?).into();
                    };
                    Ok(Res::Ok)
                },
            ),
            OpCode::NotEqImmF32 {
                result,
                left,
                right,
            } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    let lhs = regs[left];
                    if likely(lhs.is_number()) {
                        regs[result] = unsafe { lhs.as_number_uncheck() != right as f64 }.into();
                    } else {
                        regs[result] = (!lhs.eqeq((right as f64).into(), ctx)?).into();
                    };
                    Ok(Res::Ok)
                },
            ),
            OpCode::NotEqEq {
                result,
                left,
                right,
            } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    let lhs = regs[left];
                    let rhs = regs[right];

                    regs[result] = (!(lhs == rhs)).into();
                    Ok(Res::Ok)
                },
            ),
            OpCode::NotEqEqImmI32 {
                result,
                left,
                right,
            } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    let lhs = regs[left];
                    regs[result] =
                        unsafe { !(lhs.is_number() && lhs.as_number_uncheck() == right as f64) }
                            .into();
                    Ok(Res::Ok)
                },
            ),
            OpCode::NotEqEqImmF32 {
                result,
                left,
                right,
            } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    let lhs = regs[left];
                    regs[result] =
                        unsafe { !(lhs.is_number() && lhs.as_number_uncheck() == right as f64) }
                            .into();
                    Ok(Res::Ok)
                },
            ),
            OpCode::BitAnd {
                result,
                left,
                right,
            } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    let lhs = regs[left];
                    let rhs = regs[right];

                    if likely(lhs.is_number() && rhs.is_number()) {
                        regs[result] = unsafe {
                            lhs.as_number_uncheck() as i32 & rhs.as_number_uncheck() as i32
                        }
                        .into();
                    } else {
                        regs[result] = lhs.bitand(rhs, ctx)?;
                    };
                    Ok(Res::Ok)
                },
            ),
            OpCode::BitAndImmI32 {
                result,
                left,
                right,
            } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    let lhs = regs[left];

                    if likely(lhs.is_number()) {
                        regs[result] =
                            unsafe { lhs.as_number_uncheck() as i32 & right as i32 }.into();
                    } else {
                        regs[result] = (lhs.to_i32(ctx)? & right as i32).into();
                    };
                    Ok(Res::Ok)
                },
            ),
            OpCode::BitOr {
                result,
                left,
                right,
            } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    let lhs = regs[left];
                    let rhs = regs[right];

                    if likely(lhs.is_number() && rhs.is_number()) {
                        regs[result] = unsafe {
                            lhs.as_number_uncheck() as i32 | rhs.as_number_uncheck() as i32
                        }
                        .into();
                    } else {
                        regs[result] = lhs.bitor(rhs, ctx)?;
                    };
                    Ok(Res::Ok)
                },
            ),
            OpCode::BitOrImmI32 {
                result,
                left,
                right,
            } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    let lhs = regs[left];

                    if likely(lhs.is_number()) {
                        regs[result] =
                            unsafe { lhs.as_number_uncheck() as i32 | right as i32 }.into();
                    } else {
                        regs[result] = (lhs.to_i32(ctx)? | right as i32).into();
                    };
                    Ok(Res::Ok)
                },
            ),
            OpCode::BitXor {
                result,
                left,
                right,
            } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    let lhs = regs[left];
                    let rhs = regs[right];

                    if likely(lhs.is_number() && rhs.is_number()) {
                        regs[result] = unsafe {
                            lhs.as_number_uncheck() as i32 ^ rhs.as_number_uncheck() as i32
                        }
                        .into();
                    } else {
                        regs[result] = lhs.bitxor(rhs, ctx)?;
                    };
                    Ok(Res::Ok)
                },
            ),
            OpCode::BitXorImmI32 {
                result,
                left,
                right,
            } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    let lhs = regs[left];

                    if likely(lhs.is_number()) {
                        regs[result] =
                            unsafe { lhs.as_number_uncheck() as i32 ^ right as i32 }.into();
                    } else {
                        regs[result] = (lhs.to_i32(ctx)? ^ right as i32).into();
                    };
                    Ok(Res::Ok)
                },
            ),
            OpCode::BitNot { result, right } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    let rhs = regs[right];

                    if likely(rhs.is_number()) {
                        regs[result] = unsafe { !(rhs.as_number_uncheck() as i32) }.into();
                    } else {
                        regs[result] = (!rhs.to_i32(ctx)?).into();
                    };
                    Ok(Res::Ok)
                },
            ),

            OpCode::Await { result, future } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    return Ok(Res::Await(regs[future], result));
                    Ok(Res::Ok)
                },
            ),
            OpCode::Yield { result, arg } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    return Ok(Res::Yield(regs[arg], result));
                    Ok(Res::Ok)
                },
            ),
            OpCode::In {
                result,
                left,
                right,
            } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    let lhs = regs[left];
                    let rhs = regs[right];

                    if let Some(obj) = rhs.as_object() {
                        let key = lhs.to_property_key(ctx)?;
                        regs[result] = obj.has_property(key).into();
                    } else {
                        return Err(Error::TypeError(format!(
                            "Cannot use 'in' operator to search for '{}' in {}",
                            lhs.to_string(),
                            rhs.to_string()
                        ))
                        .into());
                    };
                    Ok(Res::Ok)
                },
            ),
            OpCode::PrivateIn {
                result,
                name,
                right,
            } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    let rhs = regs[right];

                    let key = state.runtime.get_field_name(name);

                    if let Some(obj) = rhs.as_object() {
                        regs[result] = obj.has_property(PropKey(name)).into();
                    } else {
                        return Err(Error::TypeError(format!(
                            "Cannot use 'in' operator to search for '#{}' in {}",
                            key,
                            rhs.to_string()
                        ))
                        .into());
                    };
                    Ok(Res::Ok)
                },
            ),
            OpCode::InstanceOf {
                result,
                left,
                right,
            } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    regs[result] = regs[left].instance_of(regs[right], ctx)?;
                    Ok(Res::Ok)
                },
            ),
            OpCode::TypeOf { result, right } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    let s = regs[right].typ().as_str();
                    regs[result] = JValue::create_static_string(s);
                    Ok(Res::Ok)
                },
            ),
            OpCode::Select { a, b, result } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    if regs[a].is_undefined() {
                        regs[result] = regs[b];
                    } else {
                        regs[result] = regs[a];
                    };
                    Ok(Res::Ok)
                },
            ),
            OpCode::CondSelect { t, a, b, result } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    if regs[t].to_bool() {
                        regs[result] = regs[a];
                    } else {
                        regs[result] = regs[b];
                    };
                    Ok(Res::Ok)
                },
            ),
            OpCode::Nullish {
                result,
                left,
                right,
            } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    let l = regs[left];

                    if l.is_undefined() || l.is_null() {
                        regs[result] = regs[right];
                    } else {
                        regs[result] = regs[left];
                    };
                    Ok(Res::Ok)
                },
            ),
            OpCode::Throw { value } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    return Err(regs[value]);
                },
            ),
            OpCode::Return { value } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    return Ok(Res::Return(regs[value]));
                },
            ),

            ///////////////////////////////////////////////////////////////////////
            //   iterator
            //////////////////////////////////////////////////////////////////////
            OpCode::PrepareForIn { target } => Box::new(
                move |state: &mut ClousureState,
                ctx: JSContext,
                regs: &mut Registers,
                this: &mut JValue,
                args: &[JValue],
                stack: &mut [JValue],
                index: &mut usize| {
                    todo!();
                    let obj = regs[target];
                    let iter = JSIterator::new(obj, JSContext { stack: ctx.stack, runtime: state.runtime })?;
                    state.iterators.push(iter);
                    Ok(Res::Ok)
                }
            ),
            OpCode::PrepareForOf { target } => Box::new(
                move |state: &mut ClousureState,
                ctx: JSContext,
                regs: &mut Registers,
                this: &mut JValue,
                args: &[JValue],
                stack: &mut [JValue],
                index: &mut usize| {
                    let obj = regs[target];
                    let iter = JSIterator::new(obj, JSContext { stack: ctx.stack, runtime: state.runtime })?;
                    state.iterators.push(iter);
                    Ok(Res::Ok)
                }
            ),
            OpCode::IterDrop => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    state.iterators.pop();
                    Ok(Res::Ok)
                },
            ),
            OpCode::IterNext {
                result,
                done,
                hint: _,
                stack_offset,
            } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    let iter = state.iterators.last_mut().unwrap();
                    let re = iter.next();
                    match re {
                        Some(v) => match v {
                            Ok(v) => regs[result] = v,
                            Err(e) => return Err(e),
                        },
                        None => {
                            regs[result] = JValue::UNDEFINED;
                            regs[done] = JValue::TRUE;
                        }
                    };
                    Ok(Res::Ok)
                },
            ),
            OpCode::IterCollect {
                result,
                stack_offset,
            } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    let iter = state.iterators.last_mut().unwrap();

                    let mut values = Vec::new();
                    for i in iter {
                        match i {
                            Ok(v) => {
                                values.push((PropFlag::THREE, v));
                            }
                            Err(e) => return Err(e),
                        };
                    }
                    let obj = JObject::with_array(values);
                    regs[result] = obj.into();
                    Ok(Res::Ok)
                },
            ),

            //////////////////////////////////////////////////////////////////
            //        memory
            //////////////////////////////////////////////////////////////////
            OpCode::Mov { from, to } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    regs[to] = regs[from];
                    Ok(Res::Ok)
                },
            ),
            OpCode::StoreTemp { value } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    state.temps.push(regs[value]);
                    Ok(Res::Ok)
                },
            ),
            OpCode::ReadTemp { value } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    regs[value] = *unsafe { state.temps.get_unchecked(state.temps.len() - 1) };
                    Ok(Res::Ok)
                },
            ),
            OpCode::ReleaseTemp => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    unsafe { state.temps.set_len(state.temps.len() - 1) };
                    Ok(Res::Ok)
                },
            ),
            OpCode::WriteToStack { from, stack_offset } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    stack[stack_offset as usize] = regs[from];
                    Ok(Res::Ok)
                },
            ),
            OpCode::ReadFromStack {
                result,
                stack_offset,
            } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    regs[result] = stack[stack_offset as usize];
                    Ok(Res::Ok)
                },
            ),
            OpCode::DeclareDynamicVar {
                from,
                kind: _,
                offset,
            } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    let v = state
                        .runtime
                        .to_mut()
                        .stack
                        .get_mut(offset as usize)
                        .unwrap();
                    *v = regs[from];
                    Ok(Res::Ok)
                },
            ),
            OpCode::WriteDynamicVar { from, id } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    state.runtime.to_mut().set_variable(id, regs[from]);
                    Ok(Res::Ok)
                },
            ),
            OpCode::ReadDynamicVar { result, id } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    match state.runtime.get_variable(id) {
                        Err(e) => return Err(e),
                        Ok(v) => {
                            regs[result] = v;
                        }
                    };
                    Ok(Res::Ok)
                },
            ),
            OpCode::WriteDynamicVarDeclared { from, offset } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    let v = unsafe {
                        state
                            .runtime
                            .to_mut()
                            .stack
                            .get_unchecked_mut(offset as usize)
                    };
                    *v = regs[from];
                    Ok(Res::Ok)
                },
            ),
            OpCode::ReadDynamicVarDeclared { result, offset } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    let v = unsafe { state.runtime.stack.get_unchecked(offset as usize) };
                    regs[result] = *v;
                    Ok(Res::Ok)
                },
            ),
            OpCode::ReadCapturedVar { result, offset } => {
                Box::new(
                    move |state: &mut ClousureState,
                          ctx: JSContext,
                          regs: &mut Registers,
                          this: &mut JValue,
                          args: &[JValue],
                          stack: &mut [JValue],
                          index: &mut usize| {
                        if let Some(c) = &mut state.capture_stack {
                            regs[result] = c[offset as usize];
                        } else {
                            #[cfg(test)]
                            panic!("reading capture variable on global context")
                        };
                        Ok(Res::Ok)
                    },
                )
            }
            OpCode::WriteCapturedVar { from, offset } => {
                Box::new(
                    move |state: &mut ClousureState,
                          ctx: JSContext,
                          regs: &mut Registers,
                          this: &mut JValue,
                          args: &[JValue],
                          stack: &mut [JValue],
                          index: &mut usize| {
                        if let Some(c) = &mut state.capture_stack {
                            c[offset as usize] = regs[from];
                        } else {
                            #[cfg(test)]
                            panic!("capture variable on global context")
                        };
                        Ok(Res::Ok)
                    },
                )
            }
            OpCode::SetThis { value } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    *this = regs[value];
                    Ok(Res::Ok)
                },
            ),

            ////////////////////////////////////////////////////////////////
            //            function
            ////////////////////////////////////////////////////////////////
            OpCode::CreateArg {
                stack_offset: _,
                len: _,
            } => Box::new(
                move |_state: &mut ClousureState,
                      _ctx: JSContext,
                      _regs: &mut Registers,
                      _this: &mut JValue,
                      _args: &[JValue],
                      _stack: &mut [JValue],
                      _index: &mut usize| { Ok(Res::Ok) },
            ),
            OpCode::PushArg {
                value,
                stack_offset,
            } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    stack[stack_offset as usize] = regs[value];
                    Ok(Res::Ok)
                },
            ),
            OpCode::PushArgSpread {
                value,
                stack_offset,
            } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    stack[stack_offset as usize] = regs[value];
                    Ok(Res::Ok)
                },
            ),
            OpCode::SpreadArg {
                base_stack_offset,
                stack_offset,
                args_len,
            } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    todo!();
                    Ok(Res::Ok)
                },
            ),
            OpCode::FinishArgs {
                base_stack_offset: _,
                len,
            } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    state.arg_len = (len as i64 + state.arg_offset_counter) as usize;
                    state.arg_offset_counter = 0;
                    Ok(Res::Ok)
                },
            ),
            OpCode::ReadParam { result, index: idx } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    if idx as usize >= args.len() {
                        regs[result] = JValue::UNDEFINED;
                    } else {
                        regs[result] = args[idx as usize];
                    };
                    Ok(Res::Ok)
                },
            ),
            OpCode::CollectParam { result, start } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    let obj = JObject::array();

                    if (start as usize) < args.len() {
                        for i in &args[start as usize..] {
                            unsafe { obj.as_array().unwrap_unchecked() }
                                .push((Default::default(), *i));
                        }
                    }
                    regs[result] = JValue::create_object(obj);
                    Ok(Res::Ok)
                },
            ),
            OpCode::Call {
                result,
                this: this_reg,
                callee,
                stack_offset,
                args_len,
            } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                        
                    let callee = regs[callee];
                    let this = regs[this_reg];
                    let args =
                        &mut stack[stack_offset as usize..args_len as usize + stack_offset as usize];
                    let stack = unsafe { args.as_mut_ptr().add(args_len as usize) };

                    if !callee.is_callable(){
                        return Err(Error::CallOnNonFunction.into())
                    }

                    if let Some(obj) = callee.as_object(){
                        let (r, err) = obj.call(state.runtime, this, args.as_mut_ptr(), args_len as usize);
                        if err{
                            return Err(r)
                        } else{
                            regs[result] = r;
                        }
                    }

                    Ok(Res::Ok)
                },
            ),
            OpCode::New {
                result,
                callee,
                stack_offset,
                args_len
            } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    let stack = &mut stack[stack_offset as usize..];

                    let constructor = regs[callee];
                    let mut r = Default::default();
                    operations::invoke_new(
                        constructor,
                        state.runtime,
                        stack.as_mut_ptr(),
                        state.arg_len,
                        &mut r
                    );
                    let v = r.0;
                    let err = r.1;
                    if err {
                        return Err(v);
                    }
                    regs[result] = v;
                    Ok(Res::Ok)
                },
            ),
            OpCode::NewTarget { result } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    regs[result] = operations::new_target(&state.runtime);
                    Ok(Res::Ok)
                },
            ),
            OpCode::ImportMeta { result } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    regs[result] = operations::import_meta(&state.runtime);
                    Ok(Res::Ok)
                },
            ),

            ////////////////////////////////////////////////////////////////////////
            //             blocks
            ////////////////////////////////////////////////////////////////////////
            OpCode::CreateBlock(_b) => {
                Box::new(move |state, ctx, regs, this, args, stack, index| {
                    //self.blocks.insert(b, None);
                    Ok(Res::Ok)
                })
            }
            OpCode::SwitchToBlock(b) => {
                Box::new(move |state, ctx, regs, this, args, stack, index| {
                    //self.insert_block(b, *index)
                    Ok(Res::Ok)
                })
            }
            OpCode::Jump { to, line } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    *index = line as usize;
                    return Ok(Res::Ok);
                    Ok(Res::Ok)
                },
            ),
            OpCode::JumpIfFalse { value, to, line } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    if !regs[value].to_bool() {
                        *index = line as usize;
                        return Ok(Res::Ok);
                    };
                    Ok(Res::Ok)
                },
            ),
            OpCode::JumpIfTrue { value, to, line } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    if regs[value].to_bool() {
                        *index = line as usize;
                        return Ok(Res::Ok);
                    };
                    Ok(Res::Ok)
                },
            ),
            OpCode::EnterTry { catch_block, line } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    state.catch_block.push((catch_block, line));
                    Ok(Res::Ok)
                },
            ),
            OpCode::ExitTry => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    state.catch_block.pop();
                    Ok(Res::Ok)
                },
            ),

            ////////////////////////////////////////////////////////////////
            //             statics
            ////////////////////////////////////////////////////////////////
            OpCode::LoadFalse { result } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    regs[result] = JValue::FALSE;
                    Ok(Res::Ok)
                },
            ),
            OpCode::LoadTrue { result } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    regs[result] = JValue::TRUE;
                    Ok(Res::Ok)
                },
            ),
            OpCode::LoadNull { result } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    regs[result] = JValue::NULL;
                    Ok(Res::Ok)
                },
            ),
            OpCode::LoadUndefined { result } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    regs[result] = JValue::UNDEFINED;
                    Ok(Res::Ok)
                },
            ),
            OpCode::LoadThis { result } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    regs[result] = *this;
                    Ok(Res::Ok)
                },
            ),
            OpCode::LoadStaticBigInt { result, id } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    regs[result] = state.runtime.get_unamed_constant(id);
                    Ok(Res::Ok)
                },
            ),
            OpCode::LoadStaticBigInt32 { result, value } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    regs[result] = JValue::create_bigint(value as i64);
                    Ok(Res::Ok)
                },
            ),
            OpCode::LoadStaticFloat { result, id } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    regs[result] = state.runtime.get_unamed_constant(id);
                    Ok(Res::Ok)
                },
            ),
            OpCode::LoadStaticFloat32 { result, value } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    regs[result] = JValue::create_number(value as f64);
                    Ok(Res::Ok)
                },
            ),
            OpCode::LoadStaticString { result, id } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    let s = state.runtime.get_string(id);
                    regs[result] = JValue::create_string(state.runtime.allocate_string(s));
                    Ok(Res::Ok)
                },
            ),

            //////////////////////////////////////////////////////////////////
            //                 object
            //////////////////////////////////////////////////////////////////
            OpCode::ReadField {
                obj,
                field,
                result,
                stack_offset,
            } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    let obj = regs[obj];
                    let field = regs[field].to_string();
                    let id = state.runtime.register_field_name(&field);
                    let stack = &mut stack[stack_offset as usize..];

                    regs[result] = obj.get_property(
                        PropKey(id),
                        JSContext {
                            stack: stack.as_mut_ptr(),
                            runtime: state.runtime,
                        },
                    )?;
                    Ok(Res::Ok)
                },
            ),
            OpCode::ReadFieldStatic {
                obj,
                result,
                field_id,
            } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    let obj = regs[obj];

                    regs[result] = obj.get_property(PropKey(field_id), ctx)?;
                    Ok(Res::Ok)
                },
            ),
            OpCode::WriteField {
                obj,
                field,
                value,
                stack_offset,
            } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    let obj = regs[obj];
                    let field = regs[field].to_string();
                    let id = state.runtime.register_field_name(&field);
                    let stack = &mut stack[stack_offset as usize..];

                    obj.set_property(
                        PropKey(id),
                        regs[value],
                        JSContext {
                            stack: stack.as_mut_ptr(),
                            runtime: state.runtime,
                        },
                    )?;
                    Ok(Res::Ok)
                },
            ),
            OpCode::WriteFieldStatic {
                obj,
                value,
                field_id,
            } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    let obj = regs[obj];
                    let value = regs[value];

                    let re = obj.set_property(PropKey(field_id), value, ctx);
                    match re {
                        Ok(()) => {}
                        Err(e) => return Err(e),
                    };
                    Ok(Res::Ok)
                },
            ),
            OpCode::RemoveFieldStatic { obj, field_id } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    let obj = regs[obj];
                    if let Some(obj) = obj.as_object() {
                        obj.remove_property(PropKey(field_id));
                    };
                    Ok(Res::Ok)
                },
            ),
            OpCode::ReadSuperField {
                constructor,
                result,
                field,
                stack_offset,
            } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    let stack = &mut stack[stack_offset as usize..];
                    let mut r = Default::default();
                    unsafe {
                        operations::super_prop(
                            &state.runtime,
                            regs[constructor],
                            regs[field],
                            stack.as_mut_ptr(),
                            &mut r
                        )
                    };
                    let v = r.0;
                    let err = r.1;
                    if err {
                        return Err(v);
                    }
                    regs[result] = v;
                    Ok(Res::Ok)
                },
            ),
            OpCode::ReadSuperFieldStatic {
                constructor,
                result,
                field_id,
            } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                        let mut r = Default::default();
                    unsafe {
                        operations::super_prop_static(
                            &state.runtime,
                            regs[constructor],
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
                    regs[result] = v;
                    Ok(Res::Ok)
                },
            ),
            OpCode::WriteSuperField {
                constructor,
                value,
                field,
            } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                        let mut r = Default::default();
                    unsafe {
                        operations::super_write_prop(
                            &state.runtime,
                            regs[constructor],
                            regs[field],
                            regs[value],
                            ctx.stack,
                            &mut r
                        )
                    };
                    let v = r.0;
                    let err = r.1;
                    if err {
                        return Err(v);
                    };
                    Ok(Res::Ok)
                },
            ),
            OpCode::WriteSuperFieldStatic {
                constructor,
                value,
                field,
            } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                        let mut r = Default::default();
                    unsafe {
                        operations::super_write_prop_static(
                            &state.runtime,
                            regs[constructor],
                            field,
                            regs[value],
                            ctx.stack,
                            &mut r
                        )
                    };
                    let v = r.0;
                    let err = r.1;
                    if err {
                        return Err(v);
                    };
                    Ok(Res::Ok)
                },
            ),
            OpCode::BindGetter {
                obj,
                field_id,
                getter,
            } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    let obj = regs[obj];
                    if let Some(obj) = obj.as_object() {
                        unsafe {
                            obj.bind_getter(PropKey(field_id), regs[getter].as_object().unwrap())
                        }
                    };
                    Ok(Res::Ok)
                },
            ),
            OpCode::BindSetter {
                obj,
                field_id,
                setter,
            } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    let obj = regs[obj];
                    if let Some(obj) = obj.as_object() {
                        obj.bind_setter(PropKey(field_id), regs[setter].as_object().unwrap());
                    };
                    Ok(Res::Ok)
                },
            ),
            OpCode::CloneObject { obj, result } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    let obj = regs[obj];
                    if let Some(obj) = obj.as_object() {
                        regs[result] = JValue::create_object(unsafe { obj.deep_clone() });
                    };
                    Ok(Res::Ok)
                },
            ),
            OpCode::ExtendObject { obj, from } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    unsafe { operations::extend_object(regs[obj], regs[from], state.runtime) };
                    Ok(Res::Ok)
                },
            ),

            ////////////////////////////////////////////////////////////////
            //           creations
            ///////////////////////////////////////////////////////////////
            OpCode::CreateArray { 
                result,
                stack_offset
             } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {

                    let e = &mut stack[stack_offset as usize..stack_offset as usize + state.arg_len];
                    let e = e.iter().map(|v|(Default::default(), *v)).collect();
                    let array = JObject::with_array(e);
                    regs[result] = JValue::create_object(array);
                    Ok(Res::Ok)
                },
            ),
            OpCode::CreateArrow {
                result,
                this: this_reg,
                id,
            } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    let f = if state.is_global {
                        unsafe {
                            state
                                .runtime
                                .get_function(id)
                                .unwrap_unchecked()
                                .create_instance(Some(regs[this_reg]))
                        }
                    } else {
                        unsafe {
                            let ptr = state.capture_stack.as_mut().unwrap_unchecked().as_mut_ptr();
                            state
                                .runtime
                                .get_function(id)
                                .unwrap_unchecked()
                                .create_instance_with_capture(
                                    Some(regs[this_reg]),
                                    state.cap.clone().unwrap(),
                                )
                        }
                    };

                    regs[result] = JValue::create_object(JObject::with_function(f));
                    Ok(Res::Ok)
                },
            ),
            OpCode::CreateFunction { result, id } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    let f = unsafe {
                        if state.is_global {
                            state
                                .runtime
                                .get_function(id)
                                .unwrap_unchecked()
                                .create_instance(None)
                        } else {
                            state
                                .runtime
                                .get_function(id)
                                .unwrap()
                                .create_instance_with_capture(None, state.cap.clone().unwrap())
                        }
                    };

                    regs[result] = JValue::create_object(JObject::with_function(f));
                    Ok(Res::Ok)
                },
            ),
            OpCode::CreateObject { result } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    regs[result] = JValue::create_object(JObject::new());
                    Ok(Res::Ok)
                },
            ),
            OpCode::CreateRegExp { result, reg_id } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    let r = state.runtime.get_regex(reg_id);
                    regs[result] = JObject::with_regex(r).into();
                    Ok(Res::Ok)
                },
            ),
            OpCode::CreateTemplate {
                result,
                id,
                stack_offset,
            } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    let stack = &mut stack[stack_offset as usize..];

                    regs[result] = unsafe {
                        operations::create_template(
                            id.0,
                            stack.as_mut_ptr(),
                            state.arg_len as u32,
                            false,
                        )
                    };
                    Ok(Res::Ok)
                },
            ),
            OpCode::CreateClass { result, class_id } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| {
                    let class = state.runtime.get_class(class_id);

                    let obj = if state.is_global {
                        class.create_without_capture()
                    } else {
                        unsafe { class.create_with_capture(state.cap.clone().unwrap()) }
                    };

                    regs[result] = obj.into();
                    Ok(Res::Ok)
                },
            ),
            OpCode::ClassBindSuper { class, super_ } => Box::new(
                move |state: &mut ClousureState,
                      ctx: JSContext,
                      regs: &mut Registers,
                      this: &mut JValue,
                      args: &[JValue],
                      stack: &mut [JValue],
                      index: &mut usize| unsafe {
                    let super_class = regs[super_];
                    let c = regs[class];
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
                        };
                    }
                    Ok(Res::Ok)
                },
            ),
        }
    }
}
