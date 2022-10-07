use std::sync::Arc;

use cranelift::codegen::ir::{self, types, SigRef, Signature, StackSlot};
use cranelift::frontend::FunctionBuilder;
use cranelift::frontend::FunctionBuilderContext;
use cranelift::frontend::Variable;
use cranelift::prelude::{isa, AbiParam, InstBuilder, MemFlags, StackSlotData, StackSlotKind};

use lock_api::RwLock as _;
use parking_lot::RwLock;

use crate::bytecodes::{LoopHint, OpCode, TempAllocValue};
use crate::fast_iter::FastIterator;
use crate::runtime::Runtime;
use crate::types::{JTypeVtable, JValue};

lazy_static::lazy_static! {
    static ref ISA:Box<dyn isa::TargetIsa> = {
        use cranelift::codegen::settings::Configurable;

        let mut flag_builder = cranelift::codegen::settings::builder();
        // On at least AArch64, "colocated" calls use shorter-range relocations,
        // which might not reach all definitions; we can't handle that here, so
        // we require long-range relocation types.
        flag_builder.set("use_colocated_libcalls", "false").unwrap();
        flag_builder.set("is_pic", "true").unwrap();
        let isa_builder = cranelift_native::builder().unwrap_or_else(|msg| {
            panic!("host machine is not supported: {}", msg);
        });
        match isa_builder.finish(cranelift::codegen::settings::Flags::new(flag_builder)){
            Ok(v) => v,
            Err(e) => panic!("{}", e)
        }
    };
}
static NOT_USE_FUNCS: RwLock<Vec<&'static mut ir::Function>> = RwLock::new(Vec::new());
static NOT_USE_FUNC_CTXS: RwLock<Vec<&'static mut FunctionBuilderContext>> =
    RwLock::new(Vec::new());

fn get_ctx() -> &'static mut FunctionBuilderContext {
    if let Some(v) = NOT_USE_FUNC_CTXS.write().pop() {
        v
    } else {
        let ctx = FunctionBuilderContext::new();
        Box::leak(Box::new(ctx))
    }
}

fn get_func() -> &'static mut ir::Function {
    if let Some(v) = NOT_USE_FUNCS.write().pop() {
        v
    } else {
        let func = ir::Function::new();
        Box::leak(Box::new(func))
    }
}

fn return_ctx_func(ctx: &'static mut FunctionBuilderContext, func: &'static mut ir::Function) {
    NOT_USE_FUNCS.write().push(func);
    NOT_USE_FUNC_CTXS.write().push(ctx);
}

/// a JValue is stored in two variables since it is twice the pointer size.
#[derive(Clone, Copy)]
struct JSVariable(Variable, Variable);

struct CallResult {
    value: ir::Value,
    vtable: ir::Value,
    error: ir::Value,
}

#[cfg(target_pointer_width = "16")]
const POINTER_TYPE: types::Type = types::I16;

#[cfg(target_pointer_width = "32")]
const POINTER_TYPE: types::Type = types::I32;

#[cfg(target_pointer_width = "64")]
const POINTER_TYPE: types::Type = types::I64;

#[cfg(target_pointer_width = "128")]
const POINTER_TYPE: types::Type = types::I128;

#[cfg(target_pointer_width = "256")]
const POINTER_TYPE: types::Type = types::I256;

pub struct JSFunctionBuilder {
    builder: FunctionBuilder<'static>,
    builder_context: &'static mut FunctionBuilderContext,

    block_offset: i32,

    this: JSVariable,
    /// pointer to runtime
    runtime: Variable,
    /// pointer to profiler
    profiler: Variable,

    next_var: u32,

    args_pointer: Vec<Variable>,
    args_len: Vec<Variable>,

    catch_blocks: Vec<ir::Block>,
    /// this block has argument JValue
    return_block: ir::Block,

    /// pointer to stack
    stack_pointer: Variable,
    capture_stack_pointer: Variable,

    registers: [JSVariable; 3],

    error: Variable,

    /// store pointer to keys
    for_in_iter: Vec<StackSlot>,
    /// stores fast_iter::Iterator, the last is current iterator
    iter: Vec<StackSlot>,

    ///fn new(iter: JValue, hint: LoopHint) -> &'static FastIterator
    into_iter_sig: SigRef,
    into_iter: unsafe fn(JValue, LoopHint) -> &'static mut FastIterator,

    /// fn(iter, this) -> (done, error, value)
    iter_next_sig: SigRef,
    iter_next: fn(&mut FastIterator, JValue, *mut JValue) -> (bool, bool, JValue),

    iter_drop_sig: SigRef,
    iter_drop: fn(&'static mut FastIterator),

    iter_collect_sig: SigRef,
    iter_collect: fn(&mut FastIterator, JValue, *mut JValue) -> (JValue, bool),

    /// stores a bool if iterator ended
    iter_ended: Variable,

    temp_allocs: Vec<StackSlot>,

    /// stores temporary values, the last is current value
    temp_values: Vec<JSVariable>,
    temp_recycle_var: Vec<JSVariable>,

    /// fn(JValueUnion, JValue) -> (JValue, bool)
    basic_op_sigref: SigRef,

    dynamic_get_sigref: SigRef,
    dynamic_get: fn(&Runtime, key: u32) -> (JValue, bool),
    dynamic_set_sigref: SigRef,
    dynamic_set: fn(&mut Runtime, key: u32, JValue),

    call: unsafe fn(JValue, &Runtime, JValue, *mut JValue, u32) -> (JValue, bool),
    call_sigref: SigRef,
}

impl JSFunctionBuilder {
    fn new() -> Self {
        let func_ctx = get_ctx();
        let func = get_func();

        let mut this = Self {
            builder: FunctionBuilder::new(func, unsafe { std::mem::transmute_copy(&func_ctx) }),
            builder_context: func_ctx,

            block_offset: 0,
            catch_blocks: Vec::new(),
            return_block: ir::Block::from_u32(0),

            this: JSVariable(Variable::with_u32(0), Variable::with_u32(1)),
            runtime: Variable::with_u32(2),
            profiler: Variable::with_u32(3),

            args_pointer: Vec::new(),
            args_len: Vec::new(),

            stack_pointer: Variable::with_u32(4),
            capture_stack_pointer: Variable::with_u32(5),

            registers: [
                JSVariable(Variable::with_u32(6), Variable::with_u32(7)),
                JSVariable(Variable::with_u32(8), Variable::with_u32(9)),
                JSVariable(Variable::with_u32(10), Variable::with_u32(11)),
            ],
            error: Variable::with_u32(12),

            for_in_iter: Vec::new(),
            iter: Vec::new(),

            into_iter_sig: SigRef::from_u32(0),
            into_iter: FastIterator::new,

            iter_next_sig: SigRef::from_u32(0),
            iter_next: FastIterator::next,

            iter_drop_sig: SigRef::from_u32(0),
            iter_drop: FastIterator::drop_,

            iter_collect_sig: SigRef::from_u32(0),
            iter_collect: FastIterator::collect,

            iter_ended: Variable::with_u32(13),

            temp_allocs: Vec::new(),
            temp_values: Vec::new(),
            temp_recycle_var: Vec::new(),
            next_var: 16,

            basic_op_sigref: SigRef::from_u32(0),

            dynamic_get_sigref: SigRef::from_u32(0),
            dynamic_get: Runtime::get_variable,
            dynamic_set_sigref: SigRef::from_u32(0),
            dynamic_set: Runtime::set_variable,

            call: JValue::call_raw,
            call_sigref: SigRef::from_u32(0),
        };

        this.define_var(0, POINTER_TYPE);
        this.define_var(1, POINTER_TYPE);
        this.define_var(2, POINTER_TYPE);
        this.define_var(3, POINTER_TYPE);
        this.define_var(4, POINTER_TYPE);
        this.define_var(5, POINTER_TYPE);
        this.define_var(6, POINTER_TYPE);
        this.define_var(7, POINTER_TYPE);
        this.define_var(8, POINTER_TYPE);
        this.define_var(9, POINTER_TYPE);
        this.define_var(10, POINTER_TYPE);
        this.define_var(11, POINTER_TYPE);

        this.define_var(12, types::B8);
        this.define_var(13, types::B8);

        // fn(JValueUnion, JValue) -> (JValue, bool)
        this.basic_op_sigref = this.builder.import_signature(Signature {
            params: vec![
                AbiParam::new(POINTER_TYPE),
                AbiParam::new(POINTER_TYPE),
                AbiParam::new(POINTER_TYPE),
            ],
            returns: vec![
                AbiParam::new(POINTER_TYPE),
                AbiParam::new(POINTER_TYPE),
                AbiParam::new(types::B8),
            ],
            call_conv: ISA.default_call_conv(),
        });

        this.into_iter_sig = this.builder.import_signature(Signature {
            params: vec![
                AbiParam::new(POINTER_TYPE),
                AbiParam::new(POINTER_TYPE),
                AbiParam::new(types::I8),
            ],
            returns: vec![AbiParam::new(POINTER_TYPE)],
            call_conv: ISA.default_call_conv(),
        });

        this.iter_next_sig = this.builder.import_signature(Signature {
            params: vec![
                AbiParam::new(POINTER_TYPE), // *mut Iter
                AbiParam::new(POINTER_TYPE),
                AbiParam::new(POINTER_TYPE), //JValue
                AbiParam::new(POINTER_TYPE), // stack: *mut JValue
            ],
            returns: vec![
                AbiParam::new(types::B8), // done
                AbiParam::new(types::B8), // error
                AbiParam::new(POINTER_TYPE),
                AbiParam::new(POINTER_TYPE), //JValue
            ],
            call_conv: ISA.default_call_conv(),
        });

        this.iter_collect_sig = this.builder.import_signature(Signature {
            params: vec![
                AbiParam::new(POINTER_TYPE), // *mut Iter
                AbiParam::new(POINTER_TYPE),
                AbiParam::new(POINTER_TYPE), //JValue
                AbiParam::new(POINTER_TYPE), // stack: *mut JValue
            ],
            returns: vec![
                AbiParam::new(POINTER_TYPE),
                AbiParam::new(POINTER_TYPE), //JValue
                AbiParam::new(types::B8),    // error
            ],
            call_conv: ISA.default_call_conv(),
        });

        this.iter_drop_sig = this.builder.import_signature(Signature {
            params: vec![
                AbiParam::new(POINTER_TYPE), // *mut Iter
            ],
            returns: vec![],
            call_conv: ISA.default_call_conv(),
        });

        this.dynamic_get_sigref = this.builder.import_signature(Signature {
            params: vec![
                AbiParam::new(POINTER_TYPE), // *mut Runtime
                AbiParam::new(types::I32),   // var_id
            ],
            returns: vec![
                AbiParam::new(POINTER_TYPE),
                AbiParam::new(POINTER_TYPE),
                AbiParam::new(types::B8),
            ],
            call_conv: ISA.default_call_conv(),
        });

        this.dynamic_set_sigref = this.builder.import_signature(Signature {
            params: vec![
                AbiParam::new(POINTER_TYPE), // *mut Runtime
                AbiParam::new(types::I32),   // var_id
                AbiParam::new(POINTER_TYPE),
                AbiParam::new(POINTER_TYPE), //JValue
            ],
            returns: vec![],
            call_conv: ISA.default_call_conv(),
        });

        //call_raw(self, runtime: &Runtime, this: JValue, stack: *mut JValue, argc: u32) -> (JValue, bool)
        this.call_sigref = this.builder.import_signature(Signature {
            params: vec![
                AbiParam::new(POINTER_TYPE),
                AbiParam::new(POINTER_TYPE),
                AbiParam::new(POINTER_TYPE),
                AbiParam::new(POINTER_TYPE),
                AbiParam::new(POINTER_TYPE),
                AbiParam::new(POINTER_TYPE),
                AbiParam::new(types::I32),
            ],
            returns: vec![
                AbiParam::new(POINTER_TYPE),
                AbiParam::new(POINTER_TYPE),
                AbiParam::new(types::B8),
            ],
            call_conv: ISA.default_call_conv(),
        });

        // fn(this:JValue, ctx:&Runtime, stack:*mut JValue, argc:u32, capture_stack:*mut JValue)
        this.builder.func.signature.params.extend(&[
            AbiParam::new(POINTER_TYPE),
            AbiParam::new(POINTER_TYPE),
            AbiParam::new(POINTER_TYPE),
            AbiParam::new(POINTER_TYPE),
            AbiParam::new(types::I32),
            AbiParam::new(POINTER_TYPE),
        ]);
        this.builder.func.signature.returns.extend(&[
            AbiParam::new(POINTER_TYPE),
            AbiParam::new(POINTER_TYPE), //JValue
            AbiParam::new(types::B8),    //error
        ]);

        let start = this.builder.create_block();
        this.builder.append_block_params_for_function_params(start);
        this.builder.switch_to_block(start);

        let params = this.builder.block_params(start).to_owned();

        this.store(this.this, params[0], params[1]);
        this.builder.def_var(this.runtime, params[2]);

        //let offset = this.builder.ins().uextend(POINTER_TYPE, params[4]);
        let stack_ptr = this.builder.ins().iadd(params[3], params[4]);

        this.builder.def_var(this.stack_pointer, stack_ptr);
        this.builder.def_var(this.capture_stack_pointer, params[5]);

        this.return_block = this.builder.create_block();

        this
    }

    fn define_var(&mut self, idx: u32, ty: types::Type) {
        self.builder.declare_var(Variable::with_u32(idx), ty);
    }

    fn store(&mut self, v: JSVariable, value: ir::Value, vtable: ir::Value) {
        self.builder.def_var(v.0, value);
        self.builder.def_var(v.1, vtable);
    }

    fn store_idx(&mut self, idx: u8, value: ir::Value, vtable: ir::Value) {
        let v = self.registers[idx as usize];
        self.builder.def_var(v.0, value);
        self.builder.def_var(v.1, vtable);
    }

    fn use_idx(&mut self, idx: u8) -> (ir::Value, ir::Value) {
        let v = self.registers[idx as usize];
        let a = self.builder.use_var(v.0);
        let b = self.builder.use_var(v.1);
        (a, b)
    }

    fn store_error(&mut self, error: ir::Value) {
        if self.catch_blocks.len() > 0 {
            let v = self.use_idx(0);
            self.builder
                .ins()
                .brnz(error, *self.catch_blocks.last().unwrap(), &[]);
        } else {
            let v = self.use_idx(0);
            self.builder
                .ins()
                .brnz(error, self.return_block, &[v.0, v.1]);
        }
    }

    fn call_vtable(
        &mut self,
        vtable: ir::Value,
        offset: i32,
        sigref: SigRef,
        args: &[ir::Value],
    ) -> CallResult {
        let addr = self
            .builder
            .ins()
            .load(POINTER_TYPE, MemFlags::new(), vtable, offset);
        let inst = self.builder.ins().call_indirect(sigref, addr, args);
        let re = self.builder.inst_results(inst);
        CallResult {
            value: re[0],
            vtable: re[1],
            error: re[2],
        }
    }

    fn translate_byte_codes(&mut self, codes: &[OpCode]) -> Arc<super::Function> {
        for code in codes {
            self.translate_byte_code(code);
        }
        todo!()
    }

    fn read_stack(&mut self, offset: u16) -> (ir::Value, ir::Value) {
        let stack = self.builder.use_var(self.stack_pointer);
        let offset = (offset as usize) * std::mem::size_of::<usize>() * 2;

        let v = self
            .builder
            .ins()
            .load(POINTER_TYPE, MemFlags::new(), stack, offset as i32);
        let t = self.builder.ins().load(
            POINTER_TYPE,
            MemFlags::new(),
            stack,
            (offset + std::mem::size_of::<usize>()) as i32,
        );
        (v, t)
    }

    /// caculate the next base stack pointer
    fn get_stack_pointer(&mut self, offset: u16) -> ir::Value {
        let stack = self.builder.use_var(self.stack_pointer);

        if self.args_len.len() >= 1 {
            // there is another function call going on
            let len = *self.args_len.last().unwrap();
            let len = self.builder.use_var(len);
            let len = self.builder.ins().uextend(POINTER_TYPE, len);
            let len = self.builder.ins().imul_imm(len, JValue::SIZE as i64);

            let stack = self.builder.ins().iadd(stack, len);
            if offset != 0 {
                self.builder
                    .ins()
                    .iadd_imm(stack, (offset as usize * JValue::SIZE) as i64)
            } else {
                stack
            }
        } else {
            if offset != 0 {
                self.builder
                    .ins()
                    .iadd_imm(stack, (offset as usize * JValue::SIZE) as i64)
            } else {
                stack
            }
        }
    }

    fn translate_byte_code(&mut self, code: &OpCode) {
        match code {
            OpCode::NoOp => {}
            OpCode::Debugger => {}
            //////////////////////////////////////////////////////////////
            //         BINARY OPS
            //////////////////////////////////////////////////////////////

            // call fn(JValueUnion, JValue) -> (JValue, error) from vtable
            OpCode::Add {
                result,
                left,
                right,
            } => {
                let l = self.use_idx(left.0);
                let r = self.use_idx(right.0);

                let re = self.call_vtable(
                    l.1,
                    JTypeVtable::offset_add(),
                    self.basic_op_sigref,
                    &[l.0, r.0, r.1],
                );

                self.store_error(re.error);
                self.store_idx(result.0, re.value, re.vtable);
            }

            OpCode::Sub {
                result,
                left,
                right,
            } => {
                let l = self.use_idx(left.0);
                let r = self.use_idx(right.0);

                let re = self.call_vtable(
                    l.1,
                    JTypeVtable::offset_sub(),
                    self.basic_op_sigref,
                    &[l.0, r.0, r.1],
                );

                self.store_error(re.error);
                self.store_idx(result.0, re.value, re.vtable);
            }

            OpCode::Mul {
                result,
                left,
                right,
            } => {
                let l = self.use_idx(left.0);
                let r = self.use_idx(right.0);

                let re = self.call_vtable(
                    l.1,
                    JTypeVtable::offset_mul(),
                    self.basic_op_sigref,
                    &[l.0, r.0, r.1],
                );

                self.store_error(re.error);
                self.store_idx(result.0, re.value, re.vtable);
            }

            OpCode::Div {
                result,
                left,
                right,
            } => {
                let l = self.use_idx(left.0);
                let r = self.use_idx(right.0);

                let re = self.call_vtable(
                    l.1,
                    JTypeVtable::offset_div(),
                    self.basic_op_sigref,
                    &[l.0, r.0, r.1],
                );

                self.store_error(re.error);
                self.store_idx(result.0, re.value, re.vtable);
            }

            OpCode::Rem {
                result,
                left,
                right,
            } => {
                let l = self.use_idx(left.0);
                let r = self.use_idx(right.0);

                let re = self.call_vtable(
                    l.1,
                    JTypeVtable::offset_rem(),
                    self.basic_op_sigref,
                    &[l.0, r.0, r.1],
                );

                self.store_error(re.error);
                self.store_idx(result.0, re.value, re.vtable);
            }
            OpCode::EqEq {
                result,
                left,
                right,
            } => {
                let l = self.use_idx(left.0);
                let r = self.use_idx(right.0);

                let re = self.call_vtable(
                    l.1,
                    JTypeVtable::offset_eqeq(),
                    self.basic_op_sigref,
                    &[l.0, r.0, r.1],
                );

                self.store_error(re.error);
                self.store_idx(result.0, re.value, re.vtable);
            }
            OpCode::NotEq {
                result,
                left,
                right,
            } => {
                let l = self.use_idx(left.0);
                let r = self.use_idx(right.0);

                let re = self.call_vtable(
                    l.1,
                    JTypeVtable::offset_eqeq(),
                    self.basic_op_sigref,
                    &[l.0, r.0, r.1],
                );

                self.store_error(re.error);
                self.store_idx(result.0, re.value, re.vtable);
            }
            OpCode::EqEqEq {
                result,
                left,
                right,
            } => {
                let l = self.use_idx(left.0);
                let r = self.use_idx(right.0);

                let true_vtable = self
                    .builder
                    .ins()
                    .iconst(POINTER_TYPE, &JTypeVtable::TRUE as *const _ as usize as i64);
                let false_vtable = self.builder.ins().iconst(
                    POINTER_TYPE,
                    &JTypeVtable::FALSE as *const _ as usize as i64,
                );

                let a = self
                    .builder
                    .ins()
                    .icmp(ir::condcodes::IntCC::Equal, l.0, r.0);
                let b = self
                    .builder
                    .ins()
                    .icmp(ir::condcodes::IntCC::Equal, l.1, r.1);
                let c = self.builder.ins().band(a, b);
                let vtable = self.builder.ins().select(c, true_vtable, false_vtable);

                #[cfg(target_pointer_width = "64")]
                let value = self.builder.ins().bextend(types::B64, c);
                #[cfg(target_pointer_width = "32")]
                let value = self.builder.ins().bextend(types::B32, c);

                let value = self.builder.ins().bitcast(POINTER_TYPE, value);
                self.store_idx(result.0, value, vtable);
            }
            OpCode::NotEqEq {
                result,
                left,
                right,
            } => {
                let l = self.use_idx(left.0);
                let r = self.use_idx(right.0);

                let true_vtable = self
                    .builder
                    .ins()
                    .iconst(POINTER_TYPE, &JTypeVtable::TRUE as *const _ as usize as i64);
                let false_vtable = self.builder.ins().iconst(
                    POINTER_TYPE,
                    &JTypeVtable::FALSE as *const _ as usize as i64,
                );

                let a = self
                    .builder
                    .ins()
                    .icmp(ir::condcodes::IntCC::Equal, l.0, r.0);
                let b = self
                    .builder
                    .ins()
                    .icmp(ir::condcodes::IntCC::Equal, l.1, r.1);
                let c = self.builder.ins().band_not(a, b);
                let vtable = self.builder.ins().select(c, true_vtable, false_vtable);

                #[cfg(target_pointer_width = "64")]
                let value = self.builder.ins().bextend(types::B64, c);
                #[cfg(target_pointer_width = "32")]
                let value = self.builder.ins().bextend(types::B32, c);

                let value = self.builder.ins().bitcast(POINTER_TYPE, value);
                self.store_idx(result.0, value, vtable);
            }
            OpCode::Exp {
                result,
                left,
                right,
            } => {
                let l = self.use_idx(left.0);
                let r = self.use_idx(right.0);

                let re = self.call_vtable(
                    l.1,
                    JTypeVtable::offset_exp(),
                    self.basic_op_sigref,
                    &[l.0, r.0, r.1],
                );

                self.store_error(re.error);
                self.store_idx(result.0, re.value, re.vtable);
            }
            OpCode::InstanceOf {
                result,
                left,
                right,
            } => {
                let l = self.use_idx(left.0);
                let r = self.use_idx(right.0);

                let re = self.call_vtable(
                    l.1,
                    JTypeVtable::offset_instanceOf(),
                    self.basic_op_sigref,
                    &[l.0, r.0, r.1],
                );

                self.store_error(re.error);
                self.store_idx(result.0, re.value, re.vtable);
            }
            OpCode::In {
                result,
                left,
                right,
            } => {
                let l = self.use_idx(left.0);
                let r = self.use_idx(right.0);

                // the right hand side is the caller
                let re = self.call_vtable(
                    r.1,
                    JTypeVtable::offset_In(),
                    self.basic_op_sigref,
                    &[l.0, r.0, r.1],
                );

                self.store_error(re.error);
                self.store_idx(result.0, re.value, re.vtable);
            }
            OpCode::Gt {
                result,
                left,
                right,
            } => {
                let l = self.use_idx(left.0);
                let r = self.use_idx(right.0);

                let re = self.call_vtable(
                    l.1,
                    JTypeVtable::offset_gt(),
                    self.basic_op_sigref,
                    &[l.0, r.0, r.1],
                );

                self.store_error(re.error);
                self.store_idx(result.0, re.value, re.vtable);
            }
            OpCode::GtEq {
                result,
                left,
                right,
            } => {
                let l = self.use_idx(left.0);
                let r = self.use_idx(right.0);

                let re = self.call_vtable(
                    l.1,
                    JTypeVtable::offset_gteq(),
                    self.basic_op_sigref,
                    &[l.0, r.0, r.1],
                );

                self.store_error(re.error);
                self.store_idx(result.0, re.value, re.vtable);
            }
            OpCode::Lt {
                result,
                left,
                right,
            } => {
                let l = self.use_idx(left.0);
                let r = self.use_idx(right.0);

                let re = self.call_vtable(
                    l.1,
                    JTypeVtable::offset_lt(),
                    self.basic_op_sigref,
                    &[l.0, r.0, r.1],
                );

                self.store_error(re.error);
                self.store_idx(result.0, re.value, re.vtable);
            }
            OpCode::LtEq {
                result,
                left,
                right,
            } => {
                let l = self.use_idx(left.0);
                let r = self.use_idx(right.0);

                let re = self.call_vtable(
                    l.1,
                    JTypeVtable::offset_lteq(),
                    self.basic_op_sigref,
                    &[l.0, r.0, r.1],
                );

                self.store_error(re.error);
                self.store_idx(result.0, re.value, re.vtable);
            }
            OpCode::And {
                result,
                left,
                right,
            } => {
                let l = self.use_idx(left.0);
                let r = self.use_idx(right.0);

                let true_vtable = self
                    .builder
                    .ins()
                    .iconst(POINTER_TYPE, &JTypeVtable::TRUE as *const _ as usize as i64);
                let false_vtable = self.builder.ins().iconst(
                    POINTER_TYPE,
                    &JTypeVtable::FALSE as *const _ as usize as i64,
                );

                let a = self
                    .builder
                    .ins()
                    .icmp_imm(ir::condcodes::IntCC::NotEqual, l.0, 0);
                let b = self
                    .builder
                    .ins()
                    .icmp_imm(ir::condcodes::IntCC::NotEqual, r.0, 0);
                let c = self.builder.ins().band(a, b);
                let vtable = self.builder.ins().select(c, true_vtable, false_vtable);

                #[cfg(target_pointer_width = "64")]
                let value = self.builder.ins().bextend(types::B64, c);
                #[cfg(target_pointer_width = "32")]
                let value = self.builder.ins().bextend(types::B32, c);

                let value = self.builder.ins().bitcast(POINTER_TYPE, value);
                self.store_idx(result.0, value, vtable);
            }
            OpCode::BitAnd {
                result,
                left,
                right,
            } => {
                todo!("bitand")
            }
            OpCode::BitOr {
                result,
                left,
                right,
            } => {
                todo!("bitor")
            }
            OpCode::BitXor {
                result,
                left,
                right,
            } => {
                todo!("bitXor")
            }

            ////////////////////////////////////////////////////////////////
            //       UNARY OPS
            ////////////////////////////////////////////////////////////////
            OpCode::Await { result, future } => {
                todo!("await")
            }

            OpCode::Select { a, b, result } => {
                let a = self.use_idx(a.0);
                let b = self.use_idx(b.0);

                let c = self.builder.ins().icmp_imm(
                    ir::condcodes::IntCC::Equal,
                    a.1,
                    JValue::UNDEFINED.type_pointer as *const _ as usize as i64,
                );
                let v = self.builder.ins().select(c, a.0, b.0);
                let t = self.builder.ins().select(c, a.1, b.1);
                self.store_idx(result.0, v, t);
            }
            OpCode::Return { value } => {
                let v = self.use_idx(value.0);
                let error = self.builder.ins().bconst(types::B8, false);
                self.builder.ins().return_(&[v.0, v.1, error]);
            }
            OpCode::Throw { value } => {
                let v = self.use_idx(value.0);
                let error = self.builder.ins().bconst(types::B8, true);
                self.builder.ins().return_(&[v.0, v.1, error]);
            }

            /////////////////////////////////////////////////////////////////
            //       memory
            /////////////////////////////////////////////////////////////////
            OpCode::Mov { from, to } => {
                let v = self.use_idx(from.0);
                self.store_idx(to.0, v.0, v.1)
            }
            OpCode::Capture {
                stack_offset,
                capture_stack_offset,
            } => {
                // read from offset and write to capture stack

                let ptr = self.builder.use_var(self.stack_pointer);
                let offset = (*stack_offset as usize) * JValue::SIZE;

                let value =
                    self.builder
                        .ins()
                        .load(POINTER_TYPE, MemFlags::new(), ptr, offset as i32);
                let vtable = self.builder.ins().load(
                    POINTER_TYPE,
                    MemFlags::new(),
                    ptr,
                    (offset + JValue::VALUE_SIZE) as i32,
                );

                let capture_stack = self.builder.use_var(self.capture_stack_pointer);
                let capture_offset = (*capture_stack_offset as usize) * JValue::SIZE;

                self.builder.ins().store(
                    MemFlags::new(),
                    value,
                    capture_stack,
                    capture_offset as i32,
                );
                self.builder.ins().store(
                    MemFlags::new(),
                    vtable,
                    capture_stack,
                    (capture_offset + JValue::VALUE_SIZE) as i32,
                );
            }
            OpCode::ReadDynamicVar { result, id } => {
                let callee = self
                    .builder
                    .ins()
                    .iconst(POINTER_TYPE, self.dynamic_get as *const u8 as usize as i64);
                let runtime = self.builder.use_var(self.runtime);
                let key = self.builder.ins().iconst(types::I32, *id as i64);

                let inst = self.builder.ins().call_indirect(
                    self.dynamic_get_sigref,
                    callee,
                    &[runtime, key],
                );
                let vs = self.builder.inst_results(inst).to_owned();
                self.store_error(vs[2]);
                self.store_idx(result.0, vs[0], vs[1]);
            }
            OpCode::WriteDynamicVar { from, id } => {
                let v = self.use_idx(from.0);

                let callee = self
                    .builder
                    .ins()
                    .iconst(POINTER_TYPE, self.dynamic_set as *const u8 as usize as i64);
                let runtime = self.builder.use_var(self.runtime);
                let key = self.builder.ins().iconst(types::I32, *id as i64);

                self.builder.ins().call_indirect(
                    self.dynamic_set_sigref,
                    callee,
                    &[runtime, key, v.0, v.1],
                );
            }
            OpCode::StoreTemp { value } => {
                let v = self.use_idx(value.0);
                let va = if self.temp_recycle_var.len() != 0 {
                    self.temp_recycle_var.pop().unwrap()
                } else {
                    let a = Variable::with_u32(self.next_var);
                    self.next_var += 1;
                    let b = Variable::with_u32(self.next_var);
                    self.next_var += 1;
                    self.builder.declare_var(a, POINTER_TYPE);
                    self.builder.declare_var(b, POINTER_TYPE);
                    JSVariable(a, b)
                };
                self.temp_values.push(va);
                self.builder.def_var(va.0, v.0);
                self.builder.def_var(va.1, v.1);
            }
            OpCode::ReadTemp { value } => {
                let va = self.temp_values.last().expect("read empty temp value");
                let v = self.builder.use_var(va.0);
                let t = self.builder.use_var(va.1);
                self.store_idx(value.0, v, t);
            }
            OpCode::ReleaseTemp => {
                let v = self.temp_values.pop().expect("release empty temp value");
                self.temp_recycle_var.push(v);
            }
            OpCode::TempAlloc { size } => {
                let slot = self.builder.create_sized_stack_slot(StackSlotData {
                    kind: StackSlotKind::ExplicitSlot,
                    size: *size,
                });
                self.temp_allocs.push(slot);
            }
            OpCode::StoreTempAlloc {
                offset,
                flag,
                value,
            } => {
                let slot = *self
                    .temp_allocs
                    .last()
                    .expect("store temp alloc before allocating");
                let v = self.use_idx(value.0);
                let flag = self.builder.ins().iconst(types::I8, *flag as i64);

                let off = *offset as i32 * std::mem::size_of::<TempAllocValue>() as i32;
                self.builder.ins().stack_store(flag, slot, off);
                self.builder.ins().stack_store(v.0, slot, off + 1);
                self.builder.ins().stack_store(
                    v.1,
                    slot,
                    off + 1 + std::mem::size_of::<usize>() as i32,
                );
            }
            OpCode::ReadTempAlloc { offset, result } => {
                let off = *offset as i32 * std::mem::size_of::<TempAllocValue>() as i32;
                let slot = *self
                    .temp_allocs
                    .last()
                    .expect("read temp alloc before allocating");

                let value = self.builder.ins().stack_load(POINTER_TYPE, slot, off + 1);
                let vtable = self.builder.ins().stack_load(
                    POINTER_TYPE,
                    slot,
                    off + 1 + std::mem::size_of::<usize>() as i32,
                );
                self.store_idx(result.0, value, vtable);
            }
            OpCode::TempDealloc { size } => {
                self.temp_allocs
                    .pop()
                    .expect("temp dealloc without allocating");
            }

            OpCode::WriteToStack { from, stack_offset } => {
                let ptr = self.builder.use_var(self.stack_pointer);
                let offset = *stack_offset as usize * JValue::SIZE;
                let v = self.use_idx(from.0);
                self.builder
                    .ins()
                    .store(MemFlags::new(), v.0, ptr, offset as i32);
                self.builder.ins().store(
                    MemFlags::new(),
                    v.1,
                    ptr,
                    (offset + std::mem::size_of::<usize>()) as i32,
                );
            }
            OpCode::ReadFromStack {
                result,
                stack_offset,
            } => {
                let ptr = self.builder.use_var(self.stack_pointer);
                let offset = *stack_offset as usize * JValue::SIZE;

                let value =
                    self.builder
                        .ins()
                        .load(POINTER_TYPE, MemFlags::new(), ptr, offset as i32);
                let vtable = self.builder.ins().load(
                    POINTER_TYPE,
                    MemFlags::new(),
                    ptr,
                    (offset + std::mem::size_of::<usize>()) as i32,
                );
                self.store_idx(result.0, value, vtable);
            }

            ////////////////////////////////////////////////////////////////
            //          Calls
            ////////////////////////////////////////////////////////////////
            OpCode::CreateArg {
                stack_offset,
                len: _,
            } => {
                let stack = self.builder.use_var(self.stack_pointer);

                let args = self.get_stack_pointer(*stack_offset);
                let len = self.builder.ins().iconst(types::I32, 0);

                let args_ptr = Variable::with_u32(self.next_var);
                self.next_var += 1;
                let args_len = Variable::with_u32(self.next_var);
                self.next_var += 1;

                self.builder.declare_var(args_ptr, POINTER_TYPE);
                self.builder.declare_var(args_len, types::I32);
                self.builder.def_var(args_ptr, args);
                self.builder.def_var(args_len, len);

                self.args_len.push(args_len);
                self.args_pointer.push(args_ptr);
            }
            OpCode::PushArg { value } => {
                let length = *self.args_len.last().unwrap();

                let len = self.builder.use_var(length);
                let len = self.builder.ins().iadd_imm(len, 1);
                self.builder.def_var(length, len);

                let len = self.builder.ins().uextend(POINTER_TYPE, len);
                let offset = self.builder.ins().imul_imm(len, JValue::SIZE as i64);

                let args_ptr = self.builder.use_var(*self.args_pointer.last().unwrap());
                let addr = self.builder.ins().iadd(args_ptr, offset);

                let value = self.use_idx(value.0);

                self.builder.ins().store(MemFlags::new(), value.0, addr, 0);
                self.builder
                    .ins()
                    .store(MemFlags::new(), value.1, addr, JValue::VALUE_SIZE as i32);
            }
            OpCode::PushArgSpread { value } => {
                let v = self.use_idx(value.0);
            }
            OpCode::FinishArgs => {}
            OpCode::Call {
                result,
                this,
                callee,
                stack_offset,
            } => {
                let func = self.use_idx(callee.0);
                let this = self.use_idx(this.0);

                let args = self.builder.use_var(*self.args_pointer.last().unwrap());
                let len = self.builder.use_var(*self.args_len.last().unwrap());
                let runtime = self.builder.use_var(self.runtime);

                let callee = self
                    .builder
                    .ins()
                    .iconst(POINTER_TYPE, self.call as *mut u8 as i64);
                let inst = self.builder.ins().call_indirect(
                    self.call_sigref,
                    callee,
                    &[func.0, func.1, runtime, this.0, this.1, args, len],
                );
                let re = self.builder.inst_results(inst).to_owned();
                self.store_idx(result.0, re[0], re[1]);
                self.store_error(re[2]);
            }

            ///////////////////////////////////////////////////////////////
            //      blocks
            ///////////////////////////////////////////////////////////////
            OpCode::CreateBlock(b) => {
                let bl = self.builder.create_block();
                if self.block_offset == 0 {
                    self.block_offset = bl.as_u32() as i32 - b.0 as i32;
                }
            }
            OpCode::SwitchToBlock(b) => {
                let b = ir::Block::with_number((b.0 as i32 + self.block_offset) as u32).unwrap();
                self.builder.switch_to_block(b);
            }
            OpCode::Jump { to } => {
                let block =
                    ir::Block::with_number((to.0 as i32 + self.block_offset) as u32).unwrap();
                self.builder.ins().jump(block, &[]);
            }
            /*
            OpCode::JumpIfError { to, throw_value } => {
                let block = ir::Block::with_number((to.0 as i32 + self.block_offset) as u32).unwrap();
                let c = self.builder.use_var(self.error);
                self.builder.ins().brnz(c, block, &[]);
            },
            */
            OpCode::JumpIfFalse { value, to } => {
                let v = self.use_idx(value.0);
                let block =
                    ir::Block::with_number((to.0 as i32 + self.block_offset) as u32).unwrap();
                self.builder.ins().brz(v.0, block, &[]);
            }
            OpCode::JumpIfTrue { value, to } => {
                let v = self.use_idx(value.0);
                let block =
                    ir::Block::with_number((to.0 as i32 + self.block_offset) as u32).unwrap();
                self.builder.ins().brnz(v.0, block, &[]);
            }
            OpCode::JumpIfIterDone { to } => {
                let block =
                    ir::Block::with_number((to.0 as i32 + self.block_offset) as u32).unwrap();
                let c = self.builder.use_var(self.iter_ended);
                self.builder.ins().brnz(c, block, &[]);
            }

            ////////////////////////////////////////////////////////////////
            //         iterator
            ////////////////////////////////////////////////////////////////
            OpCode::IntoIter { target, hint } => {
                let v = self.use_idx(target.0);

                let callee = self
                    .builder
                    .ins()
                    .iconst(POINTER_TYPE, self.into_iter as *const u8 as usize as i64);
                let hint = self.builder.ins().iconst(types::I8, *hint as u8 as i64);

                let inst =
                    self.builder
                        .ins()
                        .call_indirect(self.into_iter_sig, callee, &[v.0, v.1, hint]);
                let iter = self.builder.inst_results(inst)[0];

                let slot = self.builder.create_sized_stack_slot(StackSlotData {
                    kind: StackSlotKind::ExplicitSlot,
                    size: std::mem::size_of::<usize>() as u32,
                });

                self.builder.ins().stack_store(iter, slot, 0);
            }
            OpCode::IterNext {
                result,
                hint,
                stack_offset,
            } => {
                let callee = self
                    .builder
                    .ins()
                    .iconst(POINTER_TYPE, self.iter_next as *const u8 as usize as i64);

                let iter =
                    self.builder
                        .ins()
                        .stack_load(POINTER_TYPE, *self.iter.last().unwrap(), 0);
                let this = (
                    self.builder.use_var(self.this.0),
                    self.builder.use_var(self.this.1),
                );

                let stack = self.builder.use_var(self.stack_pointer);
                let stack = self
                    .builder
                    .ins()
                    .iadd_imm(stack, (*stack_offset as usize * JValue::SIZE) as i64);

                let inst = self.builder.ins().call_indirect(
                    self.iter_next_sig,
                    callee,
                    &[iter, this.0, this.1, stack],
                );
                let res = self.builder.inst_results(inst).to_owned();

                self.builder.def_var(self.iter_ended, res[0]);
                self.store_error(res[1]);
                self.store_idx(result.0, res[2], res[3]);
            }
            OpCode::IterCollect {
                result,
                stack_offset,
            } => {
                let callee = self
                    .builder
                    .ins()
                    .iconst(POINTER_TYPE, self.iter_collect as *const u8 as usize as i64);

                let iter =
                    self.builder
                        .ins()
                        .stack_load(POINTER_TYPE, *self.iter.last().unwrap(), 0);
                let this = (
                    self.builder.use_var(self.this.0),
                    self.builder.use_var(self.this.1),
                );

                let stack = self.builder.use_var(self.stack_pointer);
                let stack = self
                    .builder
                    .ins()
                    .iadd_imm(stack, (*stack_offset as usize * JValue::SIZE) as i64);

                let inst = self.builder.ins().call_indirect(
                    self.iter_collect_sig,
                    callee,
                    &[iter, this.0, this.1, stack],
                );
                let res = self.builder.inst_results(inst).to_owned();

                let ended = self.builder.ins().bconst(types::B8, true);
                self.builder.def_var(self.iter_ended, ended);
                self.store_error(res[2]);
                self.store_idx(result.0, res[0], res[1]);
            }
            OpCode::IterDrop => {
                let callee = self
                    .builder
                    .ins()
                    .iconst(POINTER_TYPE, self.iter_drop as *const u8 as usize as i64);
                let slot = self.iter.pop().unwrap();
                let iter = self.builder.ins().stack_load(POINTER_TYPE, slot, 0);

                self.builder
                    .ins()
                    .call_indirect(self.iter_drop_sig, callee, &[iter]);
            }

            ///////////////////////////////////////////////////////////////////////
            //      create
            //////////////////////////////////////////////////////////////////////
            OpCode::CreateArray { result } => {
                let temp_slot = *self.temp_allocs.last().unwrap();
                let addr = self.builder.ins().stack_addr(POINTER_TYPE, temp_slot, 0);

                todo!()
            }
            OpCode::CreateClass { result, class_id } => {
                todo!()
            }
            OpCode::CreateArrow { result, this, id } => {
                todo!()
            }
            OpCode::CreateFunction { result, id } => {
                todo!()
            }
            OpCode::CreateObject { result } => {
                todo!()
            }
            OpCode::CreateRegExp { result, reg_id } => {
                todo!()
            }
            OpCode::ClassBindSuper { class, super_ } => {
                todo!()
            }
            OpCode::CloneObject { obj, result } => {
                todo!()
            }

            OpCode::LoadFalse { result } => {
                let vtable = self.builder.ins().iconst(
                    POINTER_TYPE,
                    &JTypeVtable::FALSE as *const _ as usize as i64,
                );
                let value = self.builder.ins().iconst(POINTER_TYPE, 0);
                self.store_idx(result.0, value, vtable);
            }
            OpCode::LoadTrue { result } => {
                let vtable = self
                    .builder
                    .ins()
                    .iconst(POINTER_TYPE, &JTypeVtable::TRUE as *const _ as usize as i64);
                let value = self.builder.ins().iconst(POINTER_TYPE, 0);
                self.store_idx(result.0, value, vtable);
            }
            OpCode::LoadNull { result } => {
                let vtable = self
                    .builder
                    .ins()
                    .iconst(POINTER_TYPE, &JTypeVtable::NULL as *const _ as usize as i64);
                let value = self.builder.ins().iconst(POINTER_TYPE, 0);
                self.store_idx(result.0, value, vtable);
            }
            OpCode::LoadUndefined { result } => {
                let vtable = self.builder.ins().iconst(
                    POINTER_TYPE,
                    &JTypeVtable::UNDEFINED as *const _ as usize as i64,
                );
                let value = self.builder.ins().iconst(POINTER_TYPE, 0);
                self.store_idx(result.0, value, vtable);
            }
            OpCode::LoadThis { result } => {
                let this = (
                    self.builder.use_var(self.this.0),
                    self.builder.use_var(self.this.1),
                );
                self.store_idx(result.0, this.0, this.1);
            }
            OpCode::LoadStaticBigInt { result, id } => {
                let runtime = Runtime::current();
                let value = runtime.get_unamed_constant(*id);
                let v: [usize; 2] = unsafe { std::mem::transmute(value) };

                let value = self.builder.ins().iconst(POINTER_TYPE, v[0] as i64);
                let vtable = self.builder.ins().iconst(POINTER_TYPE, v[1] as i64);

                self.store_idx(result.0, value, vtable);
            }
            OpCode::LoadStaticBigInt32 { result, value } => {
                let vtable = self.builder.ins().iconst(
                    POINTER_TYPE,
                    &JTypeVtable::BIGINT as *const _ as usize as i64,
                );
                let value = self.builder.ins().iconst(POINTER_TYPE, *value as i64);
                self.store_idx(result.0, value, vtable);
            }
            OpCode::LoadStaticFloat { result, id } => {
                let runtime = Runtime::current();
                let value = runtime.get_unamed_constant(*id);
                let v: [usize; 2] = unsafe { std::mem::transmute(value) };

                let value = self.builder.ins().iconst(POINTER_TYPE, v[0] as i64);
                let vtable = self.builder.ins().iconst(POINTER_TYPE, v[1] as i64);

                self.store_idx(result.0, value, vtable);
            }
            OpCode::LoadStaticFloat32 { result, value } => {
                let vtable = self.builder.ins().iconst(
                    POINTER_TYPE,
                    &JTypeVtable::BIGINT as *const _ as usize as i64,
                );
                let value = self.builder.ins().iconst(POINTER_TYPE, unsafe {
                    std::mem::transmute::<_, i64>(*value as f64)
                });
                self.store_idx(result.0, value, vtable);
            }
            OpCode::LoadStaticString { result, id } => {
                todo!()
            }
            /*
            OpCode::LoadStaticSymbol { result, id } => {


                #[cfg(target_pointer_width = "64")]
                let value = self.builder.ins().iconst(POINTER_TYPE, unsafe{std::mem::transmute::<_, i64>((*id, 0u32))});
                #[cfg(target_pointer_width = "32")]
                let value = self.builder.ins().iconst(types::I32, *id as i64);

                let vtable = self.builder.ins().iconst(POINTER_TYPE, &JTypeVtable::SYMBOL as *const _ as usize as i64);
                self.store_idx(result.0, value, vtable);
            },
            */
            _ => todo!(),
        };
    }
}

impl Drop for JSFunctionBuilder {
    fn drop(&mut self) {
        return_ctx_func(
            unsafe { std::mem::transmute_copy(&self.builder_context) },
            unsafe { std::mem::transmute_copy(&self.builder.func) },
        )
    }
}
