use std::collections::HashMap;
use std::sync::Arc;

use cranelift::codegen::ir::{self, types, SigRef, Signature, StackSlot};
use cranelift::frontend::FunctionBuilder;
use cranelift::frontend::FunctionBuilderContext;
use cranelift::frontend::Variable;
use cranelift::prelude::{isa, AbiParam, InstBuilder, MemFlags, StackSlotData, StackSlotKind};

use cranelift_module::Module;
use lock_api::RwLock as _;
use parking_lot::RwLock;

use crate::bytecodes::{LoopHint, OpCode, TempAllocValue, Register};
use crate::operations;
use crate::runtime::Runtime;
use crate::types::JValue;

use super::ExitCode;

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

const JVALUE_TYPE: types::Type = types::I64;

pub struct JSFunctionBuilder {
    builder: FunctionBuilder<'static>,
    builder_context: &'static mut FunctionBuilderContext,

    blocks: HashMap<crate::bytecodes::Block, ir::Block>,

    jump_table_block: ir::Block,
    main_body_block: ir::Block,

    catch_blocks: Vec<ir::Block>,
    /// this block has argument JValue
    error_return_block: ir::Block,
    normal_return_block: ir::Block,

    this: Variable,
    /// pointer to runtime
    runtime: Variable,
    /// pointer to profiler
    profiler: Variable,

    /// pointer to stack
    stack_pointer: Variable,
    /// pointer to args
    args_pointer: Variable,
    /// pointer to capture stack
    capture_stack_pointer: Variable,

    /// length of spreaded argument
    args_offset_counter: Variable,
    /// length of arguments newly created
    num_args: Variable,

    registers: [Variable; 3],

    /// *mut u64
    async_counter: Variable,
    async_blocks: Vec<ir::Block>,

    yield_value: Variable,

    error: Variable,

    /// stores JSIterator, the last is current iterator
    iter: Vec<StackSlot>,

    /// stores a bool if iterator ended
    iter_ended: Variable,

    temp_allocs: Vec<StackSlot>,

    /// stores temporary values, the last is current value
    temp_values: Vec<StackSlot>,

    sigrefs: HashMap<&'static str, SigRef>,
}

impl JSFunctionBuilder {
    pub fn new(is_async:bool, is_generator:bool) -> Self {
        let func_ctx = get_ctx();
        let func = get_func();

        let mut this = Self {
            builder: FunctionBuilder::new(func, unsafe { std::mem::transmute_copy(&func_ctx) }),
            builder_context: func_ctx,

            blocks: Default::default(),
            jump_table_block: ir::Block::from_u32(0),
            main_body_block: ir::Block::from_u32(0),
            catch_blocks: Vec::new(),
            error_return_block: ir::Block::from_u32(0),
            normal_return_block: ir::Block::from_u32(0),

            this: Variable::with_u32(0),
            runtime: Variable::with_u32(1),
            profiler: Variable::with_u32(2),

            stack_pointer: Variable::with_u32(3),
            args_pointer: Variable::with_u32(4),
            capture_stack_pointer: Variable::with_u32(5),

            num_args: Variable::with_u32(6),
            args_offset_counter: Variable::with_u32(7),

            async_counter: Variable::with_u32(8),
            async_blocks: Vec::new(),

            registers: [
                Variable::with_u32(9),
                Variable::with_u32(10),
                Variable::with_u32(11),
            ],

            yield_value: Variable::with_u32(12),

            error: Variable::with_u32(13),

            iter: Vec::new(),

            iter_ended: Variable::with_u32(14),

            temp_allocs: Vec::new(),
            temp_values: Vec::new(),

            sigrefs: Default::default(),
        };

        this.define_var(0, POINTER_TYPE);
        this.define_var(1, POINTER_TYPE);
        this.define_var(2, POINTER_TYPE);
        this.define_var(3, POINTER_TYPE);
        this.define_var(4, POINTER_TYPE);
        this.define_var(5, POINTER_TYPE);

        this.define_var(6, POINTER_TYPE);
        this.define_var(7, POINTER_TYPE);

        // async counter
        this.define_var(8, POINTER_TYPE);
        
        // registers
        this.define_var(9, JVALUE_TYPE);
        this.define_var(10, JVALUE_TYPE);
        this.define_var(11, JVALUE_TYPE);

        // yield value
        this.define_var(12, JVALUE_TYPE);

        this.define_var(13, types::B8);
        this.define_var(14, types::B8);


        /*fn(
            this:JValue,
            ctx:&Runtime,
            args:*const JValue,
            argc:usize,
            stack:*mut JValue,
            capture_stack:*mut JValue,
            async_counter:*mut u64,
            yield_value:JValue,
            r0:JValue, r1:JValue, r2:JValue
        )
        */
        //this.builder.func.signature.call_conv = ISA.default_call_conv();
        
        this.builder.func.signature.params.extend(&[
            AbiParam::new(JVALUE_TYPE),  // this: JValue
            AbiParam::new(POINTER_TYPE), // runtime: &Runtime
            AbiParam::new(POINTER_TYPE), // args: *const Jvalue
            AbiParam::new(POINTER_TYPE), // argc
            AbiParam::new(POINTER_TYPE), // stack: *mut JValue
            AbiParam::new(POINTER_TYPE), // capture_stack: *mut Jvalue
            AbiParam::new(POINTER_TYPE), // async_counter: *mut u64
            AbiParam::new(JVALUE_TYPE),  // yield_resume: JValue
            AbiParam::new(JVALUE_TYPE),  // r0: JValue
            AbiParam::new(JVALUE_TYPE),  // r1: JValue
            AbiParam::new(JVALUE_TYPE),  // r2: JValue
        ]);
        // -> (JValue, JValue, JValue, JValue, ExitCode)
        this.builder.func.signature.returns.extend(&[
            AbiParam::new(JVALUE_TYPE), // value: JValue
            AbiParam::new(JVALUE_TYPE), // r0: JValue
            AbiParam::new(JVALUE_TYPE), // r1: JValue
            AbiParam::new(JVALUE_TYPE), // r2: JValue
            AbiParam::new(types::I8),   //error
        ]);

        let start = this.builder.create_block();
        this.jump_table_block = this.builder.create_block();
        this.main_body_block = this.builder.create_block();

        this.builder.append_block_params_for_function_params(start);
        this.builder.switch_to_block(start);

        let params = this.builder.block_params(start).to_owned();

        this.builder.def_var(this.this, params[0]);
        this.builder.def_var(this.runtime, params[1]);
        this.builder.def_var(this.args_pointer, params[2]);
        this.builder.def_var(this.num_args, params[3]);
        this.builder.def_var(this.stack_pointer, params[4]);
        this.builder.def_var(this.capture_stack_pointer, params[5]);
        this.builder.def_var(this.async_counter, params[6]);

        this.builder.def_var(this.yield_value, params[7]);

        // store the initial values of registers
        this.builder.def_var(this.registers[0], params[8]);
        this.builder.def_var(this.registers[1], params[9]);
        this.builder.def_var(this.registers[2], params[10]);

        // the block where the async and generator jumptable occours
        this.jump_table_block = this.builder.create_block();
        this.builder.ins().jump(this.jump_table_block, &[]);


        // error return block returns result as an error //////////////////////////////////////
        this.error_return_block = this.builder.create_block();
        this.builder
            .append_block_param(this.error_return_block, JVALUE_TYPE);

        this.builder.switch_to_block(this.error_return_block);

        // mark function done if when error
        if is_async || is_generator{
            let counter = this.builder.use_var(this.async_counter);
            let i = this.builder.ins().iconst(types::I64, i64::MAX);
            this.builder.ins().store(MemFlags::new(), i, counter, 0);
        }

        let error_value = this.builder.block_params(this.error_return_block)[0];

        let undefined_value = this
            .builder
            .ins()
            .iconst(JVALUE_TYPE, JValue::UNDEFINED.to_bits() as i64);

        let exit_code = this.builder.ins().iconst(types::I8, ExitCode::Error as i64);

        this.builder.ins().return_(&[
            error_value,
            undefined_value,
            undefined_value,
            undefined_value,
            exit_code,
        ]);
        ////////////////////////////////////////////////////////////////////////////

        // normal return block returns peacefully //////////////////////////////////
        this.normal_return_block = this.builder.create_block();
        this.builder
            .append_block_param(this.normal_return_block, JVALUE_TYPE);

        this.builder.switch_to_block(this.normal_return_block);

        // mark function done when return
        if is_async || is_generator{
            let counter = this.builder.use_var(this.async_counter);
            let i = this.builder.ins().iconst(types::I64, i64::MAX);
            this.builder.ins().store(MemFlags::new(), i, counter, 0);
        }

        // load the return value
        let return_value = this.builder.block_params(this.normal_return_block)[0];

        let undefined_value = this
            .builder
            .ins()
            .iconst(JVALUE_TYPE, JValue::UNDEFINED.to_bits() as i64);

        let exit_code = this
            .builder
            .ins()
            .iconst(types::I8, ExitCode::Return as i64);

        this.builder.ins().return_(&[
            return_value,
            undefined_value,
            undefined_value,
            undefined_value,
            exit_code,
        ]);
        /////////////////////////////////////////////////////////////////

        // switch to the main block
        this.builder.switch_to_block(this.main_body_block);

        this
    }

    #[inline]
    fn get_or_insert_signature(&mut self, name:&'static str, sig:Signature) -> SigRef{
        match self.sigrefs.get(name){
            Some(s) => *s,
            None => {
                let s = self.builder.import_signature(sig);
                self.sigrefs.insert(name, s);
                return s
            }
        }
    }

    fn define_var(&mut self, idx: u32, ty: types::Type) {
        self.builder.declare_var(Variable::with_u32(idx), ty);
    }

    fn read_reg<R>(&mut self, r: R) -> ir::Value
    where
        R: Into<usize>,
    {
        self.builder.use_var(self.registers[r.into()])
    }

    fn store_reg<R>(&mut self, r: R, value: ir::Value)
    where
        R: Into<usize>,
    {
        self.builder.def_var(self.registers[r.into()], value);
    }

    fn handle_error(&mut self, is_error: ir::Value, value: ir::Value) {
        if let Some(b) = self.catch_blocks.last() {

            self.builder
                .ins()
                .brnz(is_error, *b, &[value]);
                
        } else {

            self.builder
                .ins()
                .brnz(is_error, self.error_return_block, &[value]);
        }
    }

    pub fn translate_byte_codes(&mut self, codes: &[OpCode], runtime:&Runtime) -> super::Function {

        assert!(self.builder.current_block().unwrap() == self.main_body_block);

        for code in codes {
            self.translate_byte_code(*code, runtime);
        }

        if !self.builder.is_filled(){
            let undefined_value = self
                .builder
                .ins()
                .iconst(JVALUE_TYPE, JValue::UNDEFINED.to_bits() as i64);

            // normal exit
            // return undefined
            self.builder
                .ins()
                .jump(self.normal_return_block, &[undefined_value]);
        };

        // jump to this block if async function is done
        let done_block = self.builder.create_block();

        self.builder.switch_to_block(self.jump_table_block);

        // jump table stores all the break points for async function
        let mut table_data = ir::JumpTableData::new();

        // jump to main body if async counter is 0
        table_data.push_entry(self.main_body_block);

        // jump to async entries if not 0
        for i in &self.async_blocks {
            table_data.push_entry(*i)
        }

        let jt = self.builder.create_jump_table(table_data);

        // read the async counter
        let async_counter = self.builder.use_var(self.async_counter);
        let count = self
            .builder
            .ins()
            .load(types::I32, MemFlags::new(), async_counter, 0);

        // branch to done if async is finished
        self.builder.ins().br_table(count, done_block, jt);

        // the done block indicates the function is already finished
        self.builder.switch_to_block(done_block);

        let undefined_value = self
            .builder
            .ins()
            .iconst(JVALUE_TYPE, JValue::UNDEFINED.to_bits() as i64);

        let exit_code = self.builder.ins().iconst(types::I8, ExitCode::Done as i64);

        self.builder.ins().return_(&[
            undefined_value,
            undefined_value,
            undefined_value,
            undefined_value,
            exit_code,
        ]);

        // seal all blocks
        self.builder.seal_all_blocks();
        

        let mut ctx = cranelift::codegen::Context::for_function(self.builder.func.clone());

        ctx.want_disasm = true;
        let mut codes = Vec::new();
        //println!("{}", ctx.func.display());
        match ctx.compile_and_emit(ISA.as_ref(), &mut codes){
            Ok(code) => {
                //println!("{}", code.disasm.as_ref().unwrap());
            },
            Err(e) => {
                panic!("{:?}", e)
            }
        };

        //let re = ISA.compile_function(self.builder.func, false).unwrap();
        let mut mem = memmap2::MmapMut::map_anon(codes.len()).unwrap();
        mem.copy_from_slice(&codes);

        self.builder.finalize();
        self.builder.func.clear();

        let mem = mem.make_exec().unwrap();
        let f = unsafe { std::mem::transmute(mem.as_ptr()) };

        super::Function {
            profiler: Default::default(),
            func: f,
            #[cfg(test)]
            cr_func:ctx.func,
            _m: mem,
        }
    }
    
    fn is_float(&mut self, value: ir::Value) -> ir::Value{
        let tag = self.builder.ins().band_imm(value, (JValue::NAN_BITS) as i64);
        self.builder.ins().icmp_imm(ir::condcodes::IntCC::NotEqual, tag, (JValue::NAN_BITS >> 48) as i64)
    }

    fn is_object(&mut self, value:ir::Value) -> ir::Value{
        let tag = self.builder.ins().ushr_imm(value, 48);
        self.builder.ins().icmp_imm(ir::condcodes::IntCC::Equal, tag, (JValue::OBJECT_TAG >> 48) as i64)
    }

    fn is_string(&mut self, value:ir::Value) -> ir::Value{
        let tag = self.builder.ins().ushr_imm(value, 48);
        self.builder.ins().icmp_imm(ir::condcodes::IntCC::Equal, tag, (JValue::STRING_TAG >> 48) as i64)
    }

    fn is_int(&mut self, value: ir::Value) -> ir::Value{
        let tag = self.builder.ins().ushr_imm(value, 48);
        self.builder.ins().icmp_imm(ir::condcodes::IntCC::Equal, tag, (JValue::INT_TAG >> 48) as i64)
    }

    fn is_true(&mut self, value: ir::Value) -> ir::Value{
        self.builder.ins().icmp_imm(ir::condcodes::IntCC::Equal, value, JValue::TRUE_TAG as i64)
    }

    fn is_null(&mut self, value: ir::Value) -> ir::Value{
        self.builder.ins().icmp_imm(ir::condcodes::IntCC::Equal, value, JValue::NULL_TAG as i64)
    }

    fn is_undefined(&mut self, value: ir::Value) -> ir::Value{
        self.builder.ins().icmp_imm(ir::condcodes::IntCC::Equal, value, JValue::UNDEFINED_TAG as i64)
    }

    fn to_bool(&mut self, value: ir::Value) -> ir::Value{
        let v = self.builder.ins().band_imm(value, JValue::DATA_BITS as i64);
        //self.builder.ins().icmp_imm(ir::condcodes::IntCC::NotEqual, v, 0)
        v
    }

    fn binary<F0, F1>(&mut self, left:Register, right:Register, result:Register, slow_fn:fn(JValue, JValue, *mut JValue, &Runtime) -> (JValue, bool), f_body:F0, i_body:F1)
    where F0:Fn(&mut Self, ir::Value, ir::Value) -> ir::Value, F1:Fn(&mut Self, ir::Value, ir::Value) -> ir::Value{

        let exit = self.builder.create_block();
        let float_path = self.builder.create_block();
        let int_or_slow_path = self.builder.create_block();
        let int_path = self.builder.create_block();
        let slow_path = self.builder.create_block();
        
        self.builder.append_block_param(slow_path, JVALUE_TYPE);
        self.builder.append_block_param(slow_path, JVALUE_TYPE);
        self.builder.append_block_param(float_path, JVALUE_TYPE);
        self.builder.append_block_param(float_path, JVALUE_TYPE);
        self.builder.append_block_param(int_path, JVALUE_TYPE);
        self.builder.append_block_param(int_path, JVALUE_TYPE);
        self.builder.append_block_param(int_or_slow_path, JVALUE_TYPE);
        self.builder.append_block_param(int_or_slow_path, JVALUE_TYPE);
        
        let lhs = self.read_reg(left);
        let rhs = self.read_reg(right);
        
        let l_is_float = self.is_float(lhs);
        let r_is_float = self.is_float(rhs);

        let is_float = self.builder.ins().band(l_is_float, r_is_float);

        // if not is float, branch to int or slow path
        self.builder.ins().brz(is_float, int_or_slow_path, &[lhs, rhs]);
        self.builder.seal_block(int_or_slow_path);
        self.builder.ins().jump(float_path, &[lhs, rhs]);
        self.builder.seal_block(float_path);
        
        self.builder.switch_to_block(float_path);

        let lhs = self.builder.block_params(float_path)[0];
        let rhs = self.builder.block_params(float_path)[1];

        // create float body
        let re = f_body(self, lhs, rhs);

        self.store_reg(result, re);

        // jump to exit
        self.builder.ins().jump(exit, &[]);
        
        // int or slow path
        self.builder.switch_to_block(int_or_slow_path);
        
        let p = self.builder.block_params(int_or_slow_path);
        let lhs = p[0];
        let rhs = p[1];

        let l_is_int = self.is_int(lhs);
        let r_is_int = self.is_int(rhs);
        let is_int = self.builder.ins().band(l_is_int, r_is_int);

        // if not is int, branch to slow path
        self.builder.ins().brz(is_int, slow_path, &[lhs, rhs]);
        self.builder.seal_block(slow_path);
        self.builder.ins().jump(int_path, &[lhs, rhs]);
        self.builder.seal_block(int_path);

        // int path
        self.builder.switch_to_block(int_path);
        let lhs = self.builder.block_params(int_path)[0];
        let rhs = self.builder.block_params(int_path)[1];

        // create int body
        let re = i_body(self, lhs, rhs);

        self.store_reg(result, re);

        self.builder.ins().jump(exit, &[]);

        // slow path
        self.builder.switch_to_block(slow_path);

        let p = self.builder.block_params(slow_path);
        let lhs = p[0];
        let rhs = p[1];

        let stack = self.builder.use_var(self.stack_pointer);
        let runtime = self.builder.use_var(self.runtime);

        let callee = self.builder.ins().iconst(POINTER_TYPE, slow_fn as i64);

        let sig = self.get_or_insert_signature("add", Signature { 
            params: vec![AbiParam::new(JVALUE_TYPE), AbiParam::new(JVALUE_TYPE), AbiParam::new(POINTER_TYPE), AbiParam::new(POINTER_TYPE)], 
            returns: vec![AbiParam::new(JVALUE_TYPE), AbiParam::new(types::B8)], 
            call_conv: ISA.default_call_conv(), 
        });

        let inst = self.builder.ins().call_indirect(sig, callee, &[lhs, rhs, stack, runtime]);
        let re = self.builder.inst_results(inst);

        let value = re[0];
        let is_error = re[1];

        self.handle_error(is_error, value);
        self.store_reg(result, value);
        
        self.builder.ins().jump(exit, &[]);

        // switch to exit
        self.builder.seal_block(exit);
        self.builder.switch_to_block(exit);
    
    }

    fn binary_imm<F0, F1>(&mut self, left:Register, result:Register, imm:f64, slow_fn:extern "C" fn(JValue, JValue, *mut JValue, &Runtime) -> (JValue, bool), f_body:F0, i_body:F1)
    where F0:Fn(&mut Self, ir::Value) -> ir::Value, F1:Fn(&mut Self, ir::Value) -> ir::Value{

        let exit = self.builder.create_block();
        let float_path = self.builder.create_block();
        let int_or_slow_path = self.builder.create_block();
        let int_path = self.builder.create_block();
        let slow_path = self.builder.create_block();
        
        self.builder.append_block_param(int_or_slow_path, JVALUE_TYPE);
        self.builder.append_block_param(float_path, JVALUE_TYPE);
        self.builder.append_block_param(slow_path, JVALUE_TYPE);
        self.builder.append_block_param(int_path, JVALUE_TYPE);
        
        let lhs = self.read_reg(left);
        
        let is_float = self.is_float(lhs);

        // if not is float, branch to slow path
        self.builder.ins().brz(is_float, int_or_slow_path, &[lhs]);
        self.builder.seal_block(int_or_slow_path);
        self.builder.ins().jump(float_path, &[lhs]);
        self.builder.seal_block(float_path);

        // float path
        self.builder.switch_to_block(float_path);

        let lhs = self.builder.block_params(float_path)[0];

        // create float body
        let re = f_body(self, lhs);

        self.store_reg(result, re);

        // jump to exit
        self.builder.ins().jump(exit, &[]);
        
        // int_path
        self.builder.switch_to_block(int_or_slow_path);
        
        let p = self.builder.block_params(int_or_slow_path);
        let lhs = p[0];

        let is_int = self.is_int(lhs);

        // if not is int, branch to slow path
        self.builder.ins().brz(is_int, slow_path, &[lhs]);
        self.builder.seal_block(slow_path);
        self.builder.ins().jump(int_path, &[lhs]);
        self.builder.seal_block(int_path);

        // int path
        self.builder.switch_to_block(int_path);

        let lhs = self.builder.block_params(int_path)[0];
    
        // create int body
        let re = i_body(self, lhs);

        self.store_reg(result, re);

        self.builder.ins().jump(exit, &[]);

        // slow path
        self.builder.switch_to_block(slow_path);

        let p = self.builder.block_params(slow_path);
        let lhs = p[0];
        let rhs = self.builder.ins().iconst(JVALUE_TYPE, JValue::create_number(imm).to_bits() as i64);

        let stack = self.builder.use_var(self.stack_pointer);
        let runtime = self.builder.use_var(self.runtime);

        let callee = self.builder.ins().iconst(POINTER_TYPE, slow_fn as i64);

        let sig = self.get_or_insert_signature("add", Signature { 
            params: vec![AbiParam::new(JVALUE_TYPE), AbiParam::new(JVALUE_TYPE), AbiParam::new(POINTER_TYPE), AbiParam::new(POINTER_TYPE)], 
            returns: vec![AbiParam::new(JVALUE_TYPE), AbiParam::new(types::B8)], 
            call_conv: ISA.default_call_conv(), 
        });

        let inst = self.builder.ins().call_indirect(sig, callee, &[lhs, rhs, stack, runtime]);
        let re = self.builder.inst_results(inst);

        let value = re[0];
        let is_error = re[1];

        self.handle_error(is_error, value);
        self.store_reg(result, value);
        
        self.builder.ins().jump(exit, &[]);

        // switch to exit
        self.builder.seal_block(exit);
        self.builder.switch_to_block(exit);
    
    }

    fn translate_byte_code(&mut self, code: OpCode, runtime:&Runtime) {
        if self.builder.is_filled(){
            match code{
                OpCode::CreateBlock(b) => {
                    let block = self.builder.create_block();
                    self.blocks.insert(b, block);
                }
                OpCode::SwitchToBlock(b) => {
                    let block = self.blocks[&b];
                    self.builder.switch_to_block(block);
                },
                _ => return
            };
            return;
        };
        match code {
            OpCode::NoOp => {}
            OpCode::Debugger => {}
            OpCode::Span { start, len: _ } => {
                self.builder.set_srcloc(ir::SourceLoc::new(start));
            },
            OpCode::Mov { from, to } => {
                let v = self.read_reg(from);
                self.store_reg(to, v);
            }

            //////////////////////////////////////////////////////////////
            //     breaking operations
            /////////////////////////////////////////////////////////////
            //  the await and yield operation will write the async counter and return.
            
            OpCode::Await { result, future } => {
                // register an async block
                let block = self.builder.create_block();
                self.async_blocks.push(block);

                // load the pointer to counter
                let ptr = self.builder.use_var(self.async_counter);
                // load the identity of this breaking point
                let count = self
                    .builder
                    .ins()
                    .iconst(types::I32, self.async_blocks.len() as i64);

                // write the count to counter
                self.builder.ins().store(MemFlags::new(), count, ptr, 0);

                // read the future to be awaited
                let future = self.read_reg(future);

                // check if future is object
                let is_object = self.is_object(future);
                // set the yielded value to the future, to be readed by block
                self.builder.def_var(self.yield_value, future);
                // branch to block directly without returning if future is not object
                self.builder.ins().brz(is_object, block, &[]);

                // load the await exit code
                let exit_code = self.builder.ins().iconst(types::I8, ExitCode::Await as i64);
                // read all the registers and return them
                let (r0, r1, r2) = (self.read_reg(0u8), self.read_reg(1u8), self.read_reg(2u8));

                // return the future to be awaited and the registers
                self.builder.ins().return_(&[future, r0, r1, r2, exit_code]);

                // this block will be called after future resolved
                self.builder.switch_to_block(block);

                // get the resolved future result
                let value = self.builder.use_var(self.yield_value);
                self.store_reg(result, value);

            },
            OpCode::Yield { result, arg } => {
                // register block
                let block = self.builder.create_block();
                self.async_blocks.push(block);

                // load pointer to counter
                let ptr = self.builder.use_var(self.async_counter);
                // load identity of break point
                let count = self
                    .builder
                    .ins()
                    .iconst(types::I32, self.async_blocks.len() as i64);

                // write the identity to counter
                self.builder.ins().store(MemFlags::new(), count, ptr, 0);

                // get the value needed to be yielded
                let v = self.read_reg(arg);

                // load yield exit code
                let exit_code = self.builder.ins().iconst(types::I8, ExitCode::Yield as i64);

                // read all the registers and return them
                let (r0, r1, r2) = (self.read_reg(0u8), self.read_reg(1u8), self.read_reg(2u8));

                // return the yield value and registers
                self.builder.ins().return_(&[v, r0, r1, r2, exit_code]);

                // this block is called after next continuation
                self.builder.switch_to_block(block);

                // load the yield value
                let value = self.builder.use_var(self.yield_value);
                self.store_reg(result, value);

            },

            //////////////////////////////////////////////////////////////
            //         Block ops
            //////////////////////////////////////////////////////////////
            
            OpCode::CreateBlock(b) => {
                let block = self.builder.create_block();
                self.blocks.insert(b, block);
            },
            OpCode::SwitchToBlock(b) => {
                let block = self.blocks[&b];
                self.builder.switch_to_block(block);
            },
            OpCode::Jump { to, line:_ } => {
                let block = self.blocks[&to];
                self.builder.ins().jump(block, &[]);
                
            },
            OpCode::JumpIfTrue { value, to, line:_ } => {
                let exit = self.builder.create_block();

                let test_value = self.read_reg(value);
                let block = self.blocks[&to];

                let b = self.to_bool(test_value);
                self.builder.ins().brnz(b, block, &[]);
                self.builder.ins().jump(exit, &[]);
                self.builder.seal_block(exit);
                
                self.builder.switch_to_block(exit);
            },
            OpCode::JumpIfFalse { value, to, line:_ } => {
                let exit = self.builder.create_block();

                let test_value = self.read_reg(value);
                let block = self.blocks[&to];

                let b = self.to_bool(test_value);
                self.builder.ins().brz(b, block, &[]);

                self.builder.ins().jump(exit, &[]);
                self.builder.seal_block(exit);
                
                self.builder.switch_to_block(exit);
            },
            OpCode::JumpIfIterDone { to, line:_ } => {
                let exit = self.builder.create_block();

                let block = self.blocks[&to];
                let done = self.builder.use_var(self.iter_ended);

                self.builder.ins().brnz(done, block, &[]);

                self.builder.ins().jump(exit, &[]);
                self.builder.seal_block(exit);
                
                self.builder.switch_to_block(exit);
            },

            //////////////////////////////////////////////////////////////
            //         Memory
            //////////////////////////////////////////////////////////////
            
            OpCode::DeclareDynamicVar { from, kind:_, offset } => {
                let value = self.read_reg(from);
                let stack = self.builder.use_var(self.stack_pointer);
                let offset = offset as i32 * JValue::SIZE as i32;
                self.builder.ins().store(MemFlags::new(), value, stack, offset);
            },
            OpCode::ReadDynamicVarDeclared { result, offset } => {
                let base = runtime.stack.as_ptr();
                let p = self.builder.ins().iconst(POINTER_TYPE, unsafe{base.add(offset as usize)} as i64);
                let value = self.builder.ins().load(JVALUE_TYPE, MemFlags::new(), p, 0);
                self.store_reg(result, value);
            },
            OpCode::WriteDynamicVarDeclared { from, offset } => {
                let base = runtime.stack.as_ptr();
                let p = self.builder.ins().iconst(POINTER_TYPE, unsafe{base.add(offset as usize)} as i64);
                let value = self.read_reg(from);

                self.builder.ins().store(MemFlags::new(), value, p, 0);
            },
            OpCode::ReadDynamicVar { result, id } => {
                let runtime = self.builder.use_var(self.runtime);
                let id = self.builder.ins().iconst(types::I32, id as i64);
                let callee = self.builder.ins().iconst(POINTER_TYPE, operations::dynamic_get as i64);

                let sig = self.get_or_insert_signature("dynamic_get", Signature { 
                    params: vec![AbiParam::new(POINTER_TYPE), AbiParam::new(types::I32)], 
                    returns: vec![AbiParam::new(JVALUE_TYPE), AbiParam::new(types::B8)], 
                    call_conv: ISA.default_call_conv()
                });

                let inst = self.builder.ins().call_indirect(sig, callee, &[runtime, id]);
                let value = self.builder.inst_results(inst)[0];
                let is_error = self.builder.inst_results(inst)[1];

                self.handle_error(is_error, value);
                self.store_reg(result, value);
            },
            OpCode::WriteDynamicVar { from, id } => {
                let runtime = self.builder.use_var(self.runtime);
                let id = self.builder.ins().iconst(types::I32, id as i64);
                let value = self.read_reg(from);

                let callee = self.builder.ins().iconst(POINTER_TYPE, operations::dynamic_set as i64);

                let sig = self.get_or_insert_signature("dynamic_set", Signature { 
                    params: vec![AbiParam::new(POINTER_TYPE), AbiParam::new(types::I32), AbiParam::new(JVALUE_TYPE)], 
                    returns: vec![AbiParam::new(JVALUE_TYPE), AbiParam::new(types::B8)], 
                    call_conv: ISA.default_call_conv()
                });

                let inst = self.builder.ins().call_indirect(sig, callee, &[runtime, id, value]);
                let value = self.builder.inst_results(inst)[0];
                let is_error = self.builder.inst_results(inst)[1];

                self.handle_error(is_error, value);

            }
            OpCode::ReadParam { result, index } => {
                let params = self.builder.use_var(self.args_pointer);
                let offset = index as i32 * JValue::SIZE as i32;

                let value = self.builder.ins().load(JVALUE_TYPE, MemFlags::new(), params, offset);
                self.store_reg(result, value);
            },
            OpCode::ReadFromStack { result, stack_offset } => {
                let stack = self.builder.use_var(self.stack_pointer);
                let offset = stack_offset as i32 * JValue::SIZE as i32;
                let value = self.builder.ins().load(JVALUE_TYPE, MemFlags::new(), stack, offset);

                self.store_reg(result, value);
            },
            OpCode::WriteToStack { from, stack_offset } => {
                let stack = self.builder.use_var(self.stack_pointer);
                let offset = stack_offset as i32 * JValue::SIZE as i32;
                let value = self.read_reg(from);

                self.builder.ins().store(MemFlags::new(), value, stack, offset);
            },
            OpCode::Capture { stack_offset, capture_stack_offset } => {
                let stack = self.builder.use_var(self.stack_pointer);
                let offset = stack_offset as i32 * JValue::SIZE as i32;
                let value = self.builder.ins().load(JVALUE_TYPE, MemFlags::new(), stack, offset);

                let cap_stack = self.builder.use_var(self.capture_stack_pointer);
                let cap_offset = capture_stack_offset as i32 * JValue::SIZE as i32;

                self.builder.ins().store(MemFlags::new(), value, cap_stack, cap_offset);
            },
            OpCode::ReadCapturedVar { result, offset } => {
                let cap_stack = self.builder.use_var(self.capture_stack_pointer);
                let offset = offset as i32 * JValue::SIZE as i32;
                let value = self.builder.ins().load(JVALUE_TYPE, MemFlags::new(), cap_stack, offset);

                self.store_reg(result, value);
            },
            OpCode::WriteCapturedVar { from, offset } => {
                let cap_stack = self.builder.use_var(self.capture_stack_pointer);
                let offset = offset as i32 * JValue::SIZE as i32;
                let value = self.read_reg(from);

                self.builder.ins().store(MemFlags::new(), value, cap_stack, offset);
            },
            OpCode::StoreTemp { value } => {
                let value = self.read_reg(value);

                let slot = self.builder.create_sized_stack_slot(StackSlotData { 
                    kind: StackSlotKind::ExplicitSlot, 
                    size: std::mem::size_of::<JValue>() as u32, 
                });
                self.temp_values.push(slot);
                self.builder.ins().stack_store(value, slot, 0);
            },
            OpCode::ReadTemp { value:result } => {
                let slot = *self.temp_values.last().unwrap();

                let value = self.builder.ins().stack_load(JVALUE_TYPE, slot, 0);
                self.store_reg(result, value);
            },
            OpCode::ReleaseTemp => {
                self.temp_values.pop().unwrap();
            },

            //////////////////////////////////////////////////////////////
            OpCode::Return { value } => {
                let value = self.read_reg(value);
                let undefined = self.builder.ins().iconst(JVALUE_TYPE, JValue::UNDEFINED_TAG as i64);
                let code = self.builder.ins().iconst(types::I8, ExitCode::Return as i64);
                //self.builder.ins().jump(self.normal_return_block, &[value]);
                self.builder.ins().return_(&[value, undefined, undefined, undefined, code]);
            },
            
            //////////////////////////////////////////////////////////////
            //         BINARY OPS
            //////////////////////////////////////////////////////////////

            OpCode::Add { result, left, right } => {
                
                self.binary(left, right, result, operations::add, 
                    |this, lhs, rhs|{
                        // cast to float
                        let l_f = this.builder.ins().bitcast(types::F64, lhs);
                        let r_f = this.builder.ins().bitcast(types::F64, rhs);
                        // float add
                        let re = this.builder.ins().fadd(l_f, r_f);
                        this.builder.ins().bitcast(JVALUE_TYPE, re)

                    },
                    |this, lhs, rhs|{
                        // cast to int and add
                        let l = this.builder.ins().band_imm(lhs, JValue::DATA_BITS as i64);
                        let r = this.builder.ins().band_imm(rhs, JValue::DATA_BITS as i64);
                        let re = this.builder.ins().iadd(l, r);
                        // insert the int tag
                        this.builder.ins().bor_imm(re, JValue::INT_TAG as i64)
                    }
                );
            },
            OpCode::AddImmI32 { result, left, right } => {
                self.binary_imm(left, result, right as f64, operations::add, 
                    |this, lhs|{
                        // cast to float
                        let l_f = this.builder.ins().bitcast(types::F64, lhs);
                        let r_f = this.builder.ins().f64const(right as f64);
                        // float add
                        let re = this.builder.ins().fadd(l_f, r_f);
                        return this.builder.ins().bitcast(JVALUE_TYPE, re)
                    },
                    |this, lhs|{
                        // add directly
                        this.builder.ins().iadd_imm(lhs, right as i64)
                    }
                )
            },
            OpCode::AddImmU32 { result, left, right } => {
                self.binary_imm(left, result, right as f64, operations::add, 
                    |this, lhs|{
                        // cast to float
                        let l_f = this.builder.ins().bitcast(types::F64, lhs);
                        let r_f = this.builder.ins().f64const(right as f64);
                        // float add
                        let re = this.builder.ins().fadd(l_f, r_f);
                        return this.builder.ins().bitcast(JVALUE_TYPE, re)
                    }, 
                    |this, lhs|{
                        // u32 is not going to fit i32, therefore convert to f64 and add

                        // cast to int and add
                        let l = this.builder.ins().band_imm(lhs, JValue::DATA_BITS as i64);
                        
                        // convert int to float
                        let l_f = this.builder.ins().fcvt_from_sint(types::F64, l);
                        // load right float
                        let r_f = this.builder.ins().f64const(right as f64);
                        // add the floats
                        let re = this.builder.ins().fadd(l_f, r_f);
                        // cast to JValue
                        return this.builder.ins().bitcast(JVALUE_TYPE, re)
                    }
                )
            },
            OpCode::AddImmF32 { result, left, right } => {
                self.binary_imm(left, result, right as f64, operations::add, 
                    |this, lhs|{
                        // cast to float
                        let l_f = this.builder.ins().bitcast(types::F64, lhs);
                        let r_f = this.builder.ins().f64const(right as f64);
                        // float add
                        let re = this.builder.ins().fadd(l_f, r_f);
                        return this.builder.ins().bitcast(JVALUE_TYPE, re)
                    }, 
                    |this, lhs|{
                        // cast to int and add
                        let l = this.builder.ins().band_imm(lhs, JValue::DATA_BITS as i64);

                        // convert int to float
                        let l_f = this.builder.ins().fcvt_from_sint(types::F64, l);
                        // load right float
                        let r_f = this.builder.ins().f64const(right as f64);
                        // add the floats
                        let re = this.builder.ins().fadd(l_f, r_f);
                        // cast to JValue
                        return this.builder.ins().bitcast(JVALUE_TYPE, re)
                    }
                );
            },
            OpCode::Sub { result, left, right } => {
                self.binary(left, right, result, operations::sub, 
                    |this, lhs, rhs|{
                        // cast to float
                        let l_f = this.builder.ins().bitcast(types::F64, lhs);
                        let r_f = this.builder.ins().bitcast(types::F64, rhs);
                        // float add
                        let re = this.builder.ins().fsub(l_f, r_f);
                        this.builder.ins().bitcast(JVALUE_TYPE, re)
                    }, 
                    |this, lhs, rhs|{
                        // cast to float
                        let l_f = this.builder.ins().bitcast(types::F64, lhs);
                        let r_f = this.builder.ins().bitcast(types::F64, rhs);
                        // float add
                        let re = this.builder.ins().fsub(l_f, r_f);
                        this.builder.ins().bitcast(JVALUE_TYPE, re)
                    }
                );
            },
            OpCode::SubImmI32 { result, left, right } => {
                self.binary_imm(left, result, right as f64, operations::sub, 
                    |this, lhs|{
                        // cast to float
                        let l_f = this.builder.ins().bitcast(types::F64, lhs);
                        let r_f = this.builder.ins().f64const(right as f64);
                        // float add
                        let re = this.builder.ins().fsub(l_f, r_f);
                        return this.builder.ins().bitcast(JVALUE_TYPE, re)
                    },
                    |this, lhs|{
                        // sub directly
                        this.builder.ins().iadd_imm(lhs, -right as i64)
                    }
                )
            },
            OpCode::SubImmU32 { result, left, right } => {
                self.binary_imm(left, result, right as f64, operations::sub, 
                    |this, lhs|{
                        // cast to float
                        let l_f = this.builder.ins().bitcast(types::F64, lhs);
                        let r_f = this.builder.ins().f64const(right as f64);
                        // float add
                        let re = this.builder.ins().fsub(l_f, r_f);
                        return this.builder.ins().bitcast(JVALUE_TYPE, re)
                    }, 
                    |this, lhs|{
                        // u32 is not going to fit i32, therefore convert to f64 and add

                        // cast to int and add
                        let l = this.builder.ins().band_imm(lhs, JValue::DATA_BITS as i64);
                        
                        // convert int to float
                        let l_f = this.builder.ins().fcvt_from_sint(types::F64, l);
                        // load right float
                        let r_f = this.builder.ins().f64const(right as f64);
                        // add the floats
                        let re = this.builder.ins().fsub(l_f, r_f);
                        // cast to JValue
                        return this.builder.ins().bitcast(JVALUE_TYPE, re)
                    }
                )
            },
            OpCode::SubImmF32 { result, left, right } => {
                self.binary_imm(left, result, right as f64, operations::sub, 
                    |this, lhs|{
                        // cast to float
                        let l_f = this.builder.ins().bitcast(types::F64, lhs);
                        let r_f = this.builder.ins().f64const(right as f64);
                        // float add
                        let re = this.builder.ins().fsub(l_f, r_f);
                        return this.builder.ins().bitcast(JVALUE_TYPE, re)
                    }, 
                    |this, lhs|{
                        // cast to int and add
                        let l = this.builder.ins().band_imm(lhs, JValue::DATA_BITS as i64);

                        // convert int to float
                        let l_f = this.builder.ins().fcvt_from_sint(types::F64, l);
                        // load right float
                        let r_f = this.builder.ins().f64const(right as f64);
                        // add the floats
                        let re = this.builder.ins().fsub(l_f, r_f);
                        // cast to JValue
                        return this.builder.ins().bitcast(JVALUE_TYPE, re)
                    }
                );
            },
            OpCode::Mul { result, left, right } => {
                self.binary(left, right, result, operations::mul, 
                    |this, lhs, rhs|{
                        // cast to float
                        let l_f = this.builder.ins().bitcast(types::F64, lhs);
                        let r_f = this.builder.ins().bitcast(types::F64, rhs);
                        // float add
                        let re = this.builder.ins().fmul(l_f, r_f);
                        this.builder.ins().bitcast(JVALUE_TYPE, re)
                    }, 
                    |this, lhs, rhs|{
                        // cast to int and add
                        let l = this.builder.ins().band_imm(lhs, JValue::DATA_BITS as i64);
                        let r = this.builder.ins().band_imm(rhs, JValue::DATA_BITS as i64);
                        let re = this.builder.ins().imul(l, r);
                        // insert the int tag
                        this.builder.ins().bor_imm(re, JValue::INT_TAG as i64)
                    }
                );
            },
            OpCode::MulImmI32 { result, left, right } => {
                self.binary_imm(left, result, right as f64, operations::mul, 
                    |this, lhs|{
                        // cast to float
                        let l_f = this.builder.ins().bitcast(types::F64, lhs);
                        let r_f = this.builder.ins().f64const(right as f64);
                        // float add
                        let re = this.builder.ins().fmul(l_f, r_f);
                        return this.builder.ins().bitcast(JVALUE_TYPE, re)
                    },
                    |this, lhs|{
                        // cast to int and add
                        let l = this.builder.ins().band_imm(lhs, JValue::DATA_BITS as i64);
                        
                        // convert int to float
                        let l_f = this.builder.ins().fcvt_from_sint(types::F64, l);
                        // load right float
                        let r_f = this.builder.ins().f64const(right as f64);
                        // add the floats
                        let re = this.builder.ins().fmul(l_f, r_f);
                        // cast to JValue
                        return this.builder.ins().bitcast(JVALUE_TYPE, re)
                    }
                )
            },
            OpCode::MulImmU32 { result, left, right } => {
                self.binary_imm(left, result, right as f64, operations::mul, 
                    |this, lhs|{
                        // cast to float
                        let l_f = this.builder.ins().bitcast(types::F64, lhs);
                        let r_f = this.builder.ins().f64const(right as f64);
                        // float add
                        let re = this.builder.ins().fmul(l_f, r_f);
                        return this.builder.ins().bitcast(JVALUE_TYPE, re)
                    }, 
                    |this, lhs|{
                        // u32 is not going to fit i32, therefore convert to f64 and add

                        // cast to int and add
                        let l = this.builder.ins().band_imm(lhs, JValue::DATA_BITS as i64);
                        
                        // convert int to float
                        let l_f = this.builder.ins().fcvt_from_sint(types::F64, l);
                        // load right float
                        let r_f = this.builder.ins().f64const(right as f64);
                        // add the floats
                        let re = this.builder.ins().fmul(l_f, r_f);
                        // cast to JValue
                        return this.builder.ins().bitcast(JVALUE_TYPE, re)
                    }
                )
            },
            OpCode::MulImmF32 { result, left, right } => {
                self.binary_imm(left, result, right as f64, operations::mul, 
                    |this, lhs|{
                        // cast to float
                        let l_f = this.builder.ins().bitcast(types::F64, lhs);
                        let r_f = this.builder.ins().f64const(right as f64);
                        // float add
                        let re = this.builder.ins().fmul(l_f, r_f);
                        return this.builder.ins().bitcast(JVALUE_TYPE, re)
                    }, 
                    |this, lhs|{
                        // cast to int and add
                        let l = this.builder.ins().band_imm(lhs, JValue::DATA_BITS as i64);

                        // convert int to float
                        let l_f = this.builder.ins().fcvt_from_sint(types::F64, l);
                        // load right float
                        let r_f = this.builder.ins().f64const(right as f64);
                        // add the floats
                        let re = this.builder.ins().fmul(l_f, r_f);
                        // cast to JValue
                        return this.builder.ins().bitcast(JVALUE_TYPE, re)
                    }
                );
            },
            OpCode::Div { result, left, right } => {
                
                let exit = self.builder.create_block();
                let int_path = self.builder.create_block();
                let slow_path = self.builder.create_block();
                
                self.builder.append_block_param(slow_path, JVALUE_TYPE);
                self.builder.append_block_param(slow_path, JVALUE_TYPE);
                self.builder.append_block_param(int_path, JVALUE_TYPE);
                self.builder.append_block_param(int_path, JVALUE_TYPE);
                
                let lhs = self.read_reg(left);
                let rhs = self.read_reg(right);
                
                let l_is_float = self.is_float(lhs);
                let r_is_float = self.is_float(rhs);
                let both_is_float = self.builder.ins().band(l_is_float, r_is_float);

                // if not both is float, branch to slow path
                self.builder.ins().brz(both_is_float, int_path, &[lhs, rhs]);
                self.builder.seal_block(int_path);

                // cast to float
                let l_f = self.builder.ins().bitcast(types::F64, lhs);
                let r_f = self.builder.ins().bitcast(types::F64, rhs);
                // float add
                let re = self.builder.ins().fdiv(l_f, r_f);
                let re = self.builder.ins().bitcast(JVALUE_TYPE, re);

                self.store_reg(result, re);

                // jump to exit
                self.builder.ins().jump(exit, &[]);
                
                // int_path
                self.builder.switch_to_block(int_path);
                
                let p = self.builder.block_params(int_path);
                let lhs = p[0];
                let rhs = p[1];

                let l_is_int = self.is_int(lhs);
                let r_is_int = self.is_int(rhs);
                
                // check if both is int
                let both_is_int = self.builder.ins().band(l_is_int, r_is_int);

                // if not both is int, branch to slow path
                self.builder.ins().brz(both_is_int, slow_path, &[lhs, rhs]);
                self.builder.seal_block(slow_path);
                // fall through
                
                // cast to int and add
                let l = self.builder.ins().band_imm(lhs, JValue::DATA_BITS as i64);
                let r = self.builder.ins().band_imm(rhs, JValue::DATA_BITS as i64);

                let l = self.builder.ins().fcvt_from_sint(types::F64, l);
                let r = self.builder.ins().fcvt_from_sint(types::F64, r);
                let re = self.builder.ins().fdiv(l, r);
                // insert the int tag
                let re = self.builder.ins().bitcast(JVALUE_TYPE, re);

                self.store_reg(result, re);

                self.builder.ins().jump(exit, &[]);

                // slow path
                self.builder.switch_to_block(slow_path);

                let p = self.builder.block_params(slow_path);
                let lhs = p[0];
                let rhs = p[1];

                let stack = self.builder.use_var(self.stack_pointer);
                let runtime = self.builder.use_var(self.runtime);

                let callee = self.builder.ins().iconst(POINTER_TYPE, operations::div as i64);

                let sig = self.get_or_insert_signature("add", Signature { 
                    params: vec![AbiParam::new(JVALUE_TYPE), AbiParam::new(JVALUE_TYPE), AbiParam::new(POINTER_TYPE), AbiParam::new(POINTER_TYPE)], 
                    returns: vec![AbiParam::new(JVALUE_TYPE), AbiParam::new(types::B8)], 
                    call_conv: ISA.default_call_conv(), 
                });

                let inst = self.builder.ins().call_indirect(sig, callee, &[lhs, rhs, stack, runtime]);
                let re = self.builder.inst_results(inst);

                let value = re[0];
                let is_error = re[1];

                self.handle_error(is_error, value);
                self.store_reg(result, value);
                
                self.builder.ins().jump(exit, &[]);

                // switch to exit
                self.builder.seal_block(exit);
                self.builder.switch_to_block(exit);
            },
            OpCode::DivImmI32 { result, left, right } => {
                self.binary_imm(left, result, right as f64, operations::div, 
                    |this, lhs|{
                        // cast to float
                        let l_f = this.builder.ins().bitcast(types::F64, lhs);
                        let r_f = this.builder.ins().f64const(right as f64);
                        // float add
                        let re = this.builder.ins().fdiv(l_f, r_f);
                        return this.builder.ins().bitcast(JVALUE_TYPE, re)
                    },
                    |this, lhs|{
                        // cast to int and add
                        let l = this.builder.ins().band_imm(lhs, JValue::DATA_BITS as i64);
                        
                        // convert int to float
                        let l_f = this.builder.ins().fcvt_from_sint(types::F64, l);
                        // load right float
                        let r_f = this.builder.ins().f64const(right as f64);
                        // add the floats
                        let re = this.builder.ins().fdiv(l_f, r_f);
                        // cast to JValue
                        return this.builder.ins().bitcast(JVALUE_TYPE, re)
                    }
                )
            },
            OpCode::DivImmU32 { result, left, right } => {
                self.binary_imm(left, result, right as f64, operations::div, 
                    |this, lhs|{
                        // cast to float
                        let l_f = this.builder.ins().bitcast(types::F64, lhs);
                        let r_f = this.builder.ins().f64const(right as f64);
                        // float add
                        let re = this.builder.ins().fdiv(l_f, r_f);
                        return this.builder.ins().bitcast(JVALUE_TYPE, re)
                    }, 
                    |this, lhs|{
                        // u32 is not going to fit i32, therefore convert to f64 and add

                        // cast to int and add
                        let l = this.builder.ins().band_imm(lhs, JValue::DATA_BITS as i64);
                        
                        // convert int to float
                        let l_f = this.builder.ins().fcvt_from_sint(types::F64, l);
                        // load right float
                        let r_f = this.builder.ins().f64const(right as f64);
                        // add the floats
                        let re = this.builder.ins().fdiv(l_f, r_f);
                        // cast to JValue
                        return this.builder.ins().bitcast(JVALUE_TYPE, re)
                    }
                )
            },
            OpCode::DivImmF32 { result, left, right } => {
                self.binary_imm(left, result, right as f64, operations::div, 
                    |this, lhs|{
                        // cast to float
                        let l_f = this.builder.ins().bitcast(types::F64, lhs);
                        let r_f = this.builder.ins().f64const(right as f64);
                        // float add
                        let re = this.builder.ins().fdiv(l_f, r_f);
                        return this.builder.ins().bitcast(JVALUE_TYPE, re)
                    }, 
                    |this, lhs|{
                        // cast to int and add
                        let l = this.builder.ins().band_imm(lhs, JValue::DATA_BITS as i64);

                        // convert int to float
                        let l_f = this.builder.ins().fcvt_from_sint(types::F64, l);
                        // load right float
                        let r_f = this.builder.ins().f64const(right as f64);
                        // add the floats
                        let re = this.builder.ins().fdiv(l_f, r_f);
                        // cast to JValue
                        return this.builder.ins().bitcast(JVALUE_TYPE, re)
                    }
                );
            },
            OpCode::Rem { result, left, right } => {
                self.binary(left, right, result, operations::rem, 
                    |this, lhs, rhs|{
                        // cast to float
                        let l_f = this.builder.ins().bitcast(types::F64, lhs);
                        let r_f = this.builder.ins().bitcast(types::F64, rhs);

                        // a - (round(a / b) * b)
                        let c = this.builder.ins().fdiv(l_f, r_f);
                        let c = this.builder.ins().floor(c);
                        let re = this.builder.ins().fmul(c, r_f);
                        let re = this.builder.ins().fsub(l_f, re);

                        this.builder.ins().bitcast(JVALUE_TYPE, re)
                    }, 
                    |this, lhs, rhs|{
                        // cast to int and add
                        let l = this.builder.ins().band_imm(lhs, JValue::DATA_BITS as i64);
                        let r = this.builder.ins().band_imm(rhs, JValue::DATA_BITS as i64);

                        let l_f = this.builder.ins().fcvt_from_sint(types::F64, l);
                        let r_f = this.builder.ins().fcvt_from_sint(types::F64, r);
                        
                        // a - (round(a / b) * b)
                        let c = this.builder.ins().fdiv(l_f, r_f);
                        let c = this.builder.ins().floor(c);
                        let re = this.builder.ins().fmul(c, r_f);
                        let re = this.builder.ins().fsub(l_f, re);
                        

                        // insert the int tag
                        this.builder.ins().bitcast(JVALUE_TYPE, re)
                    }
                );
            },
            OpCode::RemImmI32 { result, left, right } => {
                self.binary_imm(left, result, right as f64, operations::rem, 
                    |this, lhs|{
                        // cast to float
                        let l_f = this.builder.ins().bitcast(types::F64, lhs);
                        let r_f = this.builder.ins().f64const(right as f64);
                        
                        // a - (floor(a / b) * b)
                        let c = this.builder.ins().fdiv(l_f, r_f);
                        let c = this.builder.ins().floor(c);
                        let re = this.builder.ins().fmul(c, r_f);
                        let re = this.builder.ins().fsub(l_f, re);
                        
                        return this.builder.ins().bitcast(JVALUE_TYPE, re)
                    },
                    |this, lhs|{
                        // cast to int and add
                        let l = this.builder.ins().band_imm(lhs, JValue::DATA_BITS as i64);
                        
                        // convert int to float
                        let l_f = this.builder.ins().fcvt_from_sint(types::F64, l);
                        // load right float
                        let r_f = this.builder.ins().f64const(right as f64);
                        
                        // a - (floor(a / b) * b)
                        let c = this.builder.ins().fdiv(l_f, r_f);
                        let c = this.builder.ins().floor(c);
                        let re = this.builder.ins().fmul(c, r_f);
                        let re = this.builder.ins().fsub(l_f, re);
                        
                        // cast to JValue
                        return this.builder.ins().bitcast(JVALUE_TYPE, re)
                    }
                )
            },
            OpCode::RemImmU32 { result, left, right } => {
                self.binary_imm(left, result, right as f64, operations::rem, 
                    |this, lhs|{
                        // cast to float
                        let l_f = this.builder.ins().bitcast(types::F64, lhs);
                        let r_f = this.builder.ins().f64const(right as f64);
                        
                        // a - (round(a / b) * b)
                        let c = this.builder.ins().fdiv(l_f, r_f);
                        let c = this.builder.ins().floor(c);
                        let re = this.builder.ins().fmul(c, r_f);
                        let re = this.builder.ins().fsub(l_f, re);
                        
                        return this.builder.ins().bitcast(JVALUE_TYPE, re)
                    }, 
                    |this, lhs|{
                        // u32 is not going to fit i32, therefore convert to f64 and add

                        // cast to int and add
                        let l = this.builder.ins().band_imm(lhs, JValue::DATA_BITS as i64);
                        
                        // convert int to float
                        let l_f = this.builder.ins().fcvt_from_sint(types::F64, l);
                        // load right float
                        let r_f = this.builder.ins().f64const(right as f64);
                        
                        // a - (round(a / b) * b)
                        let c = this.builder.ins().fdiv(l_f, r_f);
                        let c = this.builder.ins().floor(c);
                        let re = this.builder.ins().fmul(c, r_f);
                        let re = this.builder.ins().fsub(l_f, re);
                        
                        // cast to JValue
                        return this.builder.ins().bitcast(JVALUE_TYPE, re)
                    }
                )
            },
            OpCode::RemImmF32 { result, left, right } => {
                self.binary_imm(left, result, right as f64, operations::rem, 
                    |this, lhs|{
                        // cast to float
                        let l_f = this.builder.ins().bitcast(types::F64, lhs);
                        let r_f = this.builder.ins().f64const(right as f64);
                        
                        // a - (round(a / b) * b)
                        let c = this.builder.ins().fdiv(l_f, r_f);
                        let c = this.builder.ins().floor(c);
                        let re = this.builder.ins().fmul(c, r_f);
                        let re = this.builder.ins().fsub(l_f, re);
                        
                        return this.builder.ins().bitcast(JVALUE_TYPE, re)
                    }, 
                    |this, lhs|{
                        // cast to int and add
                        let l = this.builder.ins().band_imm(lhs, JValue::DATA_BITS as i64);

                        // convert int to float
                        let l_f = this.builder.ins().fcvt_from_sint(types::F64, l);
                        // load right float
                        let r_f = this.builder.ins().f64const(right as f64);
                        
                        // a - (floor(a / b) * b)
                        let c = this.builder.ins().fdiv(l_f, r_f);
                        let c = this.builder.ins().floor(c);
                        let re = this.builder.ins().fmul(c, r_f);
                        let re = this.builder.ins().fsub(l_f, re);

                        // cast to JValue
                        return this.builder.ins().bitcast(JVALUE_TYPE, re)
                    }
                );
            },
            OpCode::Exp { result, left, right } => {
                let lhs = self.read_reg(left);
                let rhs = self.read_reg(right);
                
                let stack = self.builder.use_var(self.stack_pointer);
                let runtime = self.builder.use_var(self.runtime);

                let callee = self.builder.ins().iconst(POINTER_TYPE, operations::pow as i64);
                
                let sig = self.get_or_insert_signature("add", Signature { 
                    params: vec![AbiParam::new(JVALUE_TYPE), AbiParam::new(JVALUE_TYPE), AbiParam::new(POINTER_TYPE), AbiParam::new(POINTER_TYPE)], 
                    returns: vec![AbiParam::new(JVALUE_TYPE), AbiParam::new(types::B8)],
                    call_conv: ISA.default_call_conv()
                });

                let inst = self.builder.ins().call_indirect(sig, callee, &[lhs, rhs, stack, runtime]);
                let value = self.builder.inst_results(inst)[0];
                let is_error = self.builder.inst_results(inst)[1];
                self.handle_error(is_error, value);
                self.store_reg(result, value);
            },
            OpCode::ExpImmI32 { result, left, right } => {
                let lhs = self.read_reg(left);
                let rhs = self.builder.ins().f64const(right as f64);
                
                let stack = self.builder.use_var(self.stack_pointer);
                let runtime = self.builder.use_var(self.runtime);

                let callee = self.builder.ins().iconst(POINTER_TYPE, operations::pow as i64);
                
                let sig = self.get_or_insert_signature("add", Signature { 
                    params: vec![AbiParam::new(JVALUE_TYPE), AbiParam::new(JVALUE_TYPE), AbiParam::new(POINTER_TYPE), AbiParam::new(POINTER_TYPE)], 
                    returns: vec![AbiParam::new(JVALUE_TYPE), AbiParam::new(types::B8)],
                    call_conv: ISA.default_call_conv()
                });

                let inst = self.builder.ins().call_indirect(sig, callee, &[lhs, rhs, stack, runtime]);
                let value = self.builder.inst_results(inst)[0];
                let is_error = self.builder.inst_results(inst)[1];
                self.handle_error(is_error, value);
                self.store_reg(result, value);
            },
            OpCode::ExpImmU32 { result, left, right } => {
                let lhs = self.read_reg(left);
                let rhs = self.builder.ins().f64const(right as f64);
                
                let stack = self.builder.use_var(self.stack_pointer);
                let runtime = self.builder.use_var(self.runtime);

                let callee = self.builder.ins().iconst(POINTER_TYPE, operations::pow as i64);
                
                let sig = self.get_or_insert_signature("add", Signature { 
                    params: vec![AbiParam::new(JVALUE_TYPE), AbiParam::new(JVALUE_TYPE), AbiParam::new(POINTER_TYPE), AbiParam::new(POINTER_TYPE)], 
                    returns: vec![AbiParam::new(JVALUE_TYPE), AbiParam::new(types::B8)],
                    call_conv: ISA.default_call_conv()
                });

                let inst = self.builder.ins().call_indirect(sig, callee, &[lhs, rhs, stack, runtime]);
                let value = self.builder.inst_results(inst)[0];
                let is_error = self.builder.inst_results(inst)[1];
                self.handle_error(is_error, value);
                self.store_reg(result, value);
            },
            OpCode::ExpImmF32 { result, left, right } => {
                let lhs = self.read_reg(left);
                let rhs = self.builder.ins().f64const(right as f64);
                
                let stack = self.builder.use_var(self.stack_pointer);
                let runtime = self.builder.use_var(self.runtime);

                let callee = self.builder.ins().iconst(POINTER_TYPE, operations::pow as i64);
                
                let sig = self.get_or_insert_signature("add", Signature { 
                    params: vec![AbiParam::new(JVALUE_TYPE), AbiParam::new(JVALUE_TYPE), AbiParam::new(POINTER_TYPE), AbiParam::new(POINTER_TYPE)], 
                    returns: vec![AbiParam::new(JVALUE_TYPE), AbiParam::new(types::B8)],
                    call_conv: ISA.default_call_conv()
                });

                let inst = self.builder.ins().call_indirect(sig, callee, &[lhs, rhs, stack, runtime]);
                let value = self.builder.inst_results(inst)[0];
                let is_error = self.builder.inst_results(inst)[1];
                self.handle_error(is_error, value);
                self.store_reg(result, value);
            },
            OpCode::RShift { result, left, right } => {
                
                let exit = self.builder.create_block();
                let int_path = self.builder.create_block();
                let slow_path = self.builder.create_block();
                
                self.builder.append_block_param(slow_path, JVALUE_TYPE);
                self.builder.append_block_param(slow_path, JVALUE_TYPE);
                self.builder.append_block_param(int_path, JVALUE_TYPE);
                self.builder.append_block_param(int_path, JVALUE_TYPE);
                
                let lhs = self.read_reg(left);
                let rhs = self.read_reg(right);
                
                let l_is_float = self.is_float(lhs);
                let r_is_float = self.is_float(rhs);
                let both_is_float = self.builder.ins().band(l_is_float, r_is_float);

                // if not both is float, branch to slow path
                self.builder.ins().brz(both_is_float, int_path, &[lhs, rhs]);
                self.builder.seal_block(int_path);

                // cast to float
                let l_f = self.builder.ins().bitcast(types::F64, lhs);
                let r_f = self.builder.ins().bitcast(types::F64, rhs);
                // convert to int
                let l = self.builder.ins().fcvt_to_sint(types::I32, l_f);
                let r = self.builder.ins().fcvt_to_sint(types::I32, r_f);

                let re = self.builder.ins().sshr(l, r);
                let re = self.builder.ins().fcvt_from_sint(types::F64, re);

                let re = self.builder.ins().bitcast(JVALUE_TYPE, re);

                self.store_reg(result, re);

                // jump to exit
                self.builder.ins().jump(exit, &[]);
                
                // int_path
                self.builder.switch_to_block(int_path);
                
                let p = self.builder.block_params(int_path);
                let lhs = p[0];
                let rhs = p[1];

                let l_is_int = self.is_int(lhs);
                let r_is_int = self.is_int(rhs);
                
                // check if both is int
                let both_is_int = self.builder.ins().band(l_is_int, r_is_int);

                // if not both is int, branch to slow path
                self.builder.ins().brz(both_is_int, slow_path, &[lhs, rhs]);
                self.builder.seal_block(slow_path);
                // fall through
                
                // cast to int and add
                let l = self.builder.ins().ireduce(types::I32, lhs);
                let r = self.builder.ins().ireduce(types::I32, rhs);

                let re = self.builder.ins().sshr(l, r);
                let re = self.builder.ins().fcvt_from_sint(types::F64, re);

                // insert the int tag
                let re = self.builder.ins().bitcast(JVALUE_TYPE, re);

                self.store_reg(result, re);

                self.builder.ins().jump(exit, &[]);

                // slow path
                self.builder.switch_to_block(slow_path);

                let p = self.builder.block_params(slow_path);
                let lhs = p[0];
                let rhs = p[1];

                let stack = self.builder.use_var(self.stack_pointer);
                let runtime = self.builder.use_var(self.runtime);

                let callee = self.builder.ins().iconst(POINTER_TYPE, operations::shr as i64);

                let sig = self.get_or_insert_signature("add", Signature { 
                    params: vec![AbiParam::new(JVALUE_TYPE), AbiParam::new(JVALUE_TYPE), AbiParam::new(POINTER_TYPE), AbiParam::new(POINTER_TYPE)], 
                    returns: vec![AbiParam::new(JVALUE_TYPE), AbiParam::new(types::B8)], 
                    call_conv: ISA.default_call_conv(), 
                });

                let inst = self.builder.ins().call_indirect(sig, callee, &[lhs, rhs, stack, runtime]);
                let re = self.builder.inst_results(inst);

                let value = re[0];
                let is_error = re[1];

                self.handle_error(is_error, value);
                self.store_reg(result, value);
                
                self.builder.ins().jump(exit, &[]);

                // switch to exit
                self.builder.seal_block(exit);
                self.builder.switch_to_block(exit);
            },
            OpCode::RShiftImmI32 { result, left, right } => {
                self.binary_imm(left, result, right as f64, operations::shr, 
                    |this, lhs|{
                        // cast to float
                        let l_f = this.builder.ins().bitcast(types::F64, lhs);
                        
                        // convert to int
                        let l = this.builder.ins().fcvt_to_sint(types::I32, l_f);

                        let re = this.builder.ins().sshr_imm(l, right as i64);
                        let re = this.builder.ins().fcvt_from_sint(types::F64, re);

                        let re = this.builder.ins().bitcast(JVALUE_TYPE, re);

                        return this.builder.ins().bitcast(JVALUE_TYPE, re)
                    },
                    |this, lhs|{
                        let l = this.builder.ins().ireduce(types::I32, lhs);

                        let re = this.builder.ins().sshr_imm(l, right as i32 as i64);
                        let re = this.builder.ins().fcvt_from_sint(types::F64, re);
                        // cast to JValue
                        return this.builder.ins().bitcast(JVALUE_TYPE, re)
                    }
                )
            },
            OpCode::RShiftImmF32 { result, left, right } => {
                self.binary_imm(left, result, right as f64, operations::shr, 
                    |this, lhs|{
                        // cast to float
                        let l_f = this.builder.ins().bitcast(types::F64, lhs);
                        
                        // convert to int
                        let l = this.builder.ins().fcvt_to_sint(types::I32, l_f);

                        let re = this.builder.ins().sshr_imm(l, right as i32 as i64);
                        let re = this.builder.ins().fcvt_from_sint(types::F64, re);

                        let re = this.builder.ins().bitcast(JVALUE_TYPE, re);

                        return this.builder.ins().bitcast(JVALUE_TYPE, re)
                    }, 
                    |this, lhs|{
                        let l = this.builder.ins().ireduce(types::I32, lhs);

                        let re = this.builder.ins().sshr_imm(l, right as i32 as i64);
                        let re = this.builder.ins().fcvt_from_sint(types::F64, re);
                        // cast to JValue
                        return this.builder.ins().bitcast(JVALUE_TYPE, re)
                    }
                );
            },
            OpCode::ZeroFillRShift { result, left, right } => {
                
                let exit = self.builder.create_block();
                let int_path = self.builder.create_block();
                let slow_path = self.builder.create_block();
                
                self.builder.append_block_param(slow_path, JVALUE_TYPE);
                self.builder.append_block_param(slow_path, JVALUE_TYPE);
                self.builder.append_block_param(int_path, JVALUE_TYPE);
                self.builder.append_block_param(int_path, JVALUE_TYPE);
                
                let lhs = self.read_reg(left);
                let rhs = self.read_reg(right);
                
                let l_is_float = self.is_float(lhs);
                let r_is_float = self.is_float(rhs);
                let both_is_float = self.builder.ins().band(l_is_float, r_is_float);

                // if not both is float, branch to slow path
                self.builder.ins().brz(both_is_float, int_path, &[lhs, rhs]);
                self.builder.seal_block(int_path);

                // cast to float
                let l_f = self.builder.ins().bitcast(types::F64, lhs);
                let r_f = self.builder.ins().bitcast(types::F64, rhs);
                // convert to int
                let l = self.builder.ins().fcvt_to_sint(types::I32, l_f);
                let r = self.builder.ins().fcvt_to_sint(types::I32, r_f);

                let re = self.builder.ins().ushr(l, r);
                let re = self.builder.ins().fcvt_from_sint(types::F64, re);

                let re = self.builder.ins().bitcast(JVALUE_TYPE, re);

                self.store_reg(result, re);

                // jump to exit
                self.builder.ins().jump(exit, &[]);
                
                // int_path
                self.builder.switch_to_block(int_path);
                
                let p = self.builder.block_params(int_path);
                let lhs = p[0];
                let rhs = p[1];

                let l_is_int = self.is_int(lhs);
                let r_is_int = self.is_int(rhs);
                
                // check if both is int
                let both_is_int = self.builder.ins().band(l_is_int, r_is_int);

                // if not both is int, branch to slow path
                self.builder.ins().brz(both_is_int, slow_path, &[lhs, rhs]);
                self.builder.seal_block(slow_path);
                // fall through
                
                // cast to int and add
                let l = self.builder.ins().ireduce(types::I32, lhs);
                let r = self.builder.ins().ireduce(types::I32, rhs);

                let re = self.builder.ins().ushr(l, r);
                let re = self.builder.ins().fcvt_from_sint(types::F64, re);

                // insert the int tag
                let re = self.builder.ins().bitcast(JVALUE_TYPE, re);

                self.store_reg(result, re);

                self.builder.ins().jump(exit, &[]);

                // slow path
                self.builder.switch_to_block(slow_path);

                let p = self.builder.block_params(slow_path);
                let lhs = p[0];
                let rhs = p[1];

                let stack = self.builder.use_var(self.stack_pointer);
                let runtime = self.builder.use_var(self.runtime);

                let callee = self.builder.ins().iconst(POINTER_TYPE, operations::zshr as i64);

                let sig = self.get_or_insert_signature("add", Signature { 
                    params: vec![AbiParam::new(JVALUE_TYPE), AbiParam::new(JVALUE_TYPE), AbiParam::new(POINTER_TYPE), AbiParam::new(POINTER_TYPE)], 
                    returns: vec![AbiParam::new(JVALUE_TYPE), AbiParam::new(types::B8)], 
                    call_conv: ISA.default_call_conv(), 
                });

                let inst = self.builder.ins().call_indirect(sig, callee, &[lhs, rhs, stack, runtime]);
                let re = self.builder.inst_results(inst);

                let value = re[0];
                let is_error = re[1];

                self.handle_error(is_error, value);
                self.store_reg(result, value);
                
                self.builder.ins().jump(exit, &[]);

                // switch to exit
                self.builder.seal_block(exit);
                self.builder.switch_to_block(exit);
            },
            OpCode::ZeroFillRShiftImmI32 { result, left, right } => {
                self.binary_imm(left, result, right as f64, operations::zshr, 
                    |this, lhs|{
                        // cast to float
                        let l_f = this.builder.ins().bitcast(types::F64, lhs);
                        
                        // convert to int
                        let l = this.builder.ins().fcvt_to_sint(types::I32, l_f);

                        let re = this.builder.ins().ushr_imm(l, right as i64);
                        let re = this.builder.ins().fcvt_from_sint(types::F64, re);

                        let re = this.builder.ins().bitcast(JVALUE_TYPE, re);

                        return this.builder.ins().bitcast(JVALUE_TYPE, re)
                    },
                    |this, lhs|{
                        let l = this.builder.ins().ireduce(types::I32, lhs);

                        let re = this.builder.ins().ushr_imm(l, right as i32 as i64);
                        let re = this.builder.ins().fcvt_from_sint(types::F64, re);
                        // cast to JValue
                        return this.builder.ins().bitcast(JVALUE_TYPE, re)
                    }
                )
            },
            OpCode::ZeroFillRShiftImmF32 { result, left, right } => {
                self.binary_imm(left, result, right as f64, operations::zshr, 
                    |this, lhs|{
                        // cast to float
                        let l_f = this.builder.ins().bitcast(types::F64, lhs);
                        
                        // convert to int
                        let l = this.builder.ins().fcvt_to_sint(types::I32, l_f);

                        let re = this.builder.ins().ushr_imm(l, right as i32 as i64);
                        let re = this.builder.ins().fcvt_from_sint(types::F64, re);

                        let re = this.builder.ins().bitcast(JVALUE_TYPE, re);

                        return this.builder.ins().bitcast(JVALUE_TYPE, re)
                    }, 
                    |this, lhs|{
                        let l = this.builder.ins().ireduce(types::I32, lhs);

                        let re = this.builder.ins().ushr_imm(l, right as i32 as i64);
                        let re = this.builder.ins().fcvt_from_sint(types::F64, re);
                        // cast to JValue
                        return this.builder.ins().bitcast(JVALUE_TYPE, re)
                    }
                );
            },
            OpCode::LShift { result, left, right } => {
                
                let exit = self.builder.create_block();
                let int_path = self.builder.create_block();
                let slow_path = self.builder.create_block();
                
                self.builder.append_block_param(slow_path, JVALUE_TYPE);
                self.builder.append_block_param(slow_path, JVALUE_TYPE);
                self.builder.append_block_param(int_path, JVALUE_TYPE);
                self.builder.append_block_param(int_path, JVALUE_TYPE);
                
                let lhs = self.read_reg(left);
                let rhs = self.read_reg(right);
                
                let l_is_float = self.is_float(lhs);
                let r_is_float = self.is_float(rhs);
                let both_is_float = self.builder.ins().band(l_is_float, r_is_float);

                // if not both is float, branch to slow path
                self.builder.ins().brz(both_is_float, int_path, &[lhs, rhs]);
                self.builder.seal_block(int_path);

                // cast to float
                let l_f = self.builder.ins().bitcast(types::F64, lhs);
                let r_f = self.builder.ins().bitcast(types::F64, rhs);
                // convert to int
                let l = self.builder.ins().fcvt_to_sint(types::I32, l_f);
                let r = self.builder.ins().fcvt_to_sint(types::I32, r_f);

                let re = self.builder.ins().ishl(l, r);
                let re = self.builder.ins().fcvt_from_sint(types::F64, re);

                let re = self.builder.ins().bitcast(JVALUE_TYPE, re);

                self.store_reg(result, re);

                // jump to exit
                self.builder.ins().jump(exit, &[]);
                
                // int_path
                self.builder.switch_to_block(int_path);
                
                let p = self.builder.block_params(int_path);
                let lhs = p[0];
                let rhs = p[1];

                let l_is_int = self.is_int(lhs);
                let r_is_int = self.is_int(rhs);
                
                // check if both is int
                let both_is_int = self.builder.ins().band(l_is_int, r_is_int);

                // if not both is int, branch to slow path
                self.builder.ins().brz(both_is_int, slow_path, &[lhs, rhs]);
                self.builder.seal_block(slow_path);
                // fall through
                
                // cast to int and add
                let l = self.builder.ins().ireduce(types::I32, lhs);
                let r = self.builder.ins().ireduce(types::I32, rhs);

                let re = self.builder.ins().ishl(l, r);
                let re = self.builder.ins().fcvt_from_sint(types::F64, re);

                // insert the int tag
                let re = self.builder.ins().bitcast(JVALUE_TYPE, re);

                self.store_reg(result, re);

                self.builder.ins().jump(exit, &[]);

                // slow path
                self.builder.switch_to_block(slow_path);

                let p = self.builder.block_params(slow_path);
                let lhs = p[0];
                let rhs = p[1];

                let stack = self.builder.use_var(self.stack_pointer);
                let runtime = self.builder.use_var(self.runtime);

                let callee = self.builder.ins().iconst(POINTER_TYPE, operations::shl as i64);

                let sig = self.get_or_insert_signature("add", Signature { 
                    params: vec![AbiParam::new(JVALUE_TYPE), AbiParam::new(JVALUE_TYPE), AbiParam::new(POINTER_TYPE), AbiParam::new(POINTER_TYPE)], 
                    returns: vec![AbiParam::new(JVALUE_TYPE), AbiParam::new(types::B8)], 
                    call_conv: ISA.default_call_conv(), 
                });

                let inst = self.builder.ins().call_indirect(sig, callee, &[lhs, rhs, stack, runtime]);
                let re = self.builder.inst_results(inst);

                let value = re[0];
                let is_error = re[1];

                self.handle_error(is_error, value);
                self.store_reg(result, value);
                
                self.builder.ins().jump(exit, &[]);

                // switch to exit
                self.builder.seal_block(exit);
                self.builder.switch_to_block(exit);
            },
            OpCode::LShiftImmI32 { result, left, right } => {
                self.binary_imm(left, result, right as f64, operations::shl, 
                    |this, lhs|{
                        // cast to float
                        let l_f = this.builder.ins().bitcast(types::F64, lhs);
                        
                        // convert to int
                        let l = this.builder.ins().fcvt_to_sint(types::I32, l_f);

                        let re = this.builder.ins().ishl_imm(l, right as i64);
                        let re = this.builder.ins().fcvt_from_sint(types::F64, re);

                        let re = this.builder.ins().bitcast(JVALUE_TYPE, re);

                        return this.builder.ins().bitcast(JVALUE_TYPE, re)
                    },
                    |this, lhs|{
                        let l = this.builder.ins().ireduce(types::I32, lhs);

                        let re = this.builder.ins().ishl_imm(l, right as i32 as i64);
                        let re = this.builder.ins().fcvt_from_sint(types::F64, re);
                        // cast to JValue
                        return this.builder.ins().bitcast(JVALUE_TYPE, re)
                    }
                )
            },
            OpCode::LShiftImmF32 { result, left, right } => {
                self.binary_imm(left, result, right as f64, operations::shl, 
                    |this, lhs|{
                        // cast to float
                        let l_f = this.builder.ins().bitcast(types::F64, lhs);
                        
                        // convert to int
                        let l = this.builder.ins().fcvt_to_sint(types::I32, l_f);

                        let re = this.builder.ins().ishl_imm(l, right as i32 as i64);
                        let re = this.builder.ins().fcvt_from_sint(types::F64, re);

                        let re = this.builder.ins().bitcast(JVALUE_TYPE, re);

                        return this.builder.ins().bitcast(JVALUE_TYPE, re)
                    }, 
                    |this, lhs|{
                        let l = this.builder.ins().ireduce(types::I32, lhs);

                        let re = this.builder.ins().ishl_imm(l, right as i32 as i64);
                        let re = this.builder.ins().fcvt_from_sint(types::F64, re);
                        // cast to JValue
                        return this.builder.ins().bitcast(JVALUE_TYPE, re)
                    }
                );
            },
            OpCode::And { result, left, right } => {
                let lhs = self.read_reg(left);
                let rhs = self.read_reg(right);

                let l = self.to_bool(lhs);
                let r = self.to_bool(rhs);
                let b = self.builder.ins().band(l, r);

                let t = self.builder.ins().iconst(JVALUE_TYPE, JValue::TRUE_TAG as i64);
                let f = self.builder.ins().iconst(JVALUE_TYPE, JValue::FALSE_TAG as i64);

                let re = self.builder.ins().select(b, t, f);
                self.store_reg(result, re);
            },
            OpCode::AndImm { result, left, right } => {
                let lhs = self.read_reg(left);

                let l = self.to_bool(lhs);

                let b = if right{
                    l
                } else{
                    self.builder.ins().band_imm(l, right as i64)
                };
                
                let t = self.builder.ins().iconst(JVALUE_TYPE, JValue::TRUE_TAG as i64);
                let f = self.builder.ins().iconst(JVALUE_TYPE, JValue::FALSE_TAG as i64);

                let re = self.builder.ins().select(b, t, f);
                self.store_reg(result, re);
            },
            OpCode::Or { result, left, right } => {
                let lhs = self.read_reg(left);
                let rhs = self.read_reg(right);

                let b = self.to_bool(lhs);
                let re = self.builder.ins().select(b, lhs, rhs);
                self.store_reg(result, re);
            },
            OpCode::EqEq { result, left, right } => {
                let lhs = self.read_reg(left);
                let rhs = self.read_reg(right);

                let exit = self.builder.create_block();
                let slow_path = self.builder.create_block();

                self.builder.append_block_param(slow_path, JVALUE_TYPE);
                self.builder.append_block_param(slow_path, JVALUE_TYPE);
                
                let t = self.builder.ins().iconst(JVALUE_TYPE, JValue::TRUE_TAG as i64);
                self.store_reg(result, t);

                let b = self.builder.ins().icmp(ir::condcodes::IntCC::Equal, lhs, rhs);
                self.builder.ins().brz(b, slow_path, &[lhs, rhs]);
                self.builder.seal_block(slow_path);

                self.builder.ins().jump(exit, &[]);

                // slow path
                self.builder.switch_to_block(slow_path);

                let lhs = self.builder.block_params(slow_path)[0];
                let rhs = self.builder.block_params(slow_path)[1];

                let stack = self.builder.use_var(self.stack_pointer);
                let runtime = self.builder.use_var(self.runtime);

                let callee = self.builder.ins().iconst(POINTER_TYPE, operations::eqeq as i64);

                let sig = self.get_or_insert_signature("add", Signature { 
                    params: vec![AbiParam::new(JVALUE_TYPE), AbiParam::new(JVALUE_TYPE), AbiParam::new(POINTER_TYPE), AbiParam::new(POINTER_TYPE)], 
                    returns: vec![AbiParam::new(JVALUE_TYPE), AbiParam::new(types::B8)], 
                    call_conv: ISA.default_call_conv(), 
                });

                let inst = self.builder.ins().call_indirect(sig, callee, &[lhs, rhs, stack, runtime]);
                let re = self.builder.inst_results(inst);

                let value = re[0];
                let is_error = re[1];

                self.handle_error(is_error, value);
                self.store_reg(result, value);

                self.builder.ins().jump(exit, &[]);

                self.builder.seal_block(exit);
                self.builder.switch_to_block(exit);
            },
            OpCode::EqEqImmI32 { result, left, right } => {
                let lhs = self.read_reg(left);

                let exit = self.builder.create_block();
                let slow_path = self.builder.create_block();

                self.builder.append_block_param(slow_path, JVALUE_TYPE);
                
                // store a true first
                let t = self.builder.ins().iconst(JVALUE_TYPE, JValue::TRUE_TAG as i64);
                self.store_reg(result, t);

                let b = self.builder.ins().icmp_imm(ir::condcodes::IntCC::Equal, lhs, JValue::create_number(right as f64).to_bits() as i64);
                // branch if not equal
                self.builder.ins().brz(b, slow_path, &[lhs]);
                self.builder.seal_block(slow_path);

                // jump to exit if true
                self.builder.ins().jump(exit, &[]);

                // slow path
                self.builder.switch_to_block(slow_path);

                let lhs = self.builder.block_params(slow_path)[0];
                let rhs = self.builder.ins().iconst(JVALUE_TYPE, JValue::create_number(right as f64).to_bits() as i64);

                let stack = self.builder.use_var(self.stack_pointer);
                let runtime = self.builder.use_var(self.runtime);

                let callee = self.builder.ins().iconst(POINTER_TYPE, operations::eqeq as i64);

                let sig = self.get_or_insert_signature("add", Signature { 
                    params: vec![AbiParam::new(JVALUE_TYPE), AbiParam::new(JVALUE_TYPE), AbiParam::new(POINTER_TYPE), AbiParam::new(POINTER_TYPE)], 
                    returns: vec![AbiParam::new(JVALUE_TYPE), AbiParam::new(types::B8)], 
                    call_conv: ISA.default_call_conv(), 
                });

                let inst = self.builder.ins().call_indirect(sig, callee, &[lhs, rhs, stack, runtime]);
                let re = self.builder.inst_results(inst);

                let value = re[0];
                let is_error = re[1];

                self.handle_error(is_error, value);
                self.store_reg(result, value);

                self.builder.ins().jump(exit, &[]);

                self.builder.seal_block(exit);
                self.builder.switch_to_block(exit);
            },
            OpCode::EqEqImmF32 { result, left, right } => {
                let lhs = self.read_reg(left);

                let exit = self.builder.create_block();
                let slow_path = self.builder.create_block();

                self.builder.append_block_param(slow_path, JVALUE_TYPE);
                
                // store a true first
                let t = self.builder.ins().iconst(JVALUE_TYPE, JValue::TRUE_TAG as i64);
                self.store_reg(result, t);

                let b = self.builder.ins().icmp_imm(ir::condcodes::IntCC::Equal, lhs, JValue::create_number(right as f64).to_bits() as i64);
                self.builder.ins().brz(b, slow_path, &[lhs]);
                self.builder.seal_block(slow_path);

                // jump to exit if true
                self.builder.ins().jump(exit, &[]);

                // slow path
                self.builder.switch_to_block(slow_path);

                let lhs = self.builder.block_params(slow_path)[0];
                let rhs = self.builder.ins().iconst(JVALUE_TYPE, JValue::create_number(right as f64).to_bits() as i64);

                let stack = self.builder.use_var(self.stack_pointer);
                let runtime = self.builder.use_var(self.runtime);

                let callee = self.builder.ins().iconst(POINTER_TYPE, operations::eqeq as i64);

                let sig = self.get_or_insert_signature("add", Signature { 
                    params: vec![AbiParam::new(JVALUE_TYPE), AbiParam::new(JVALUE_TYPE), AbiParam::new(POINTER_TYPE), AbiParam::new(POINTER_TYPE)], 
                    returns: vec![AbiParam::new(JVALUE_TYPE), AbiParam::new(types::B8)], 
                    call_conv: ISA.default_call_conv(), 
                });

                let inst = self.builder.ins().call_indirect(sig, callee, &[lhs, rhs, stack, runtime]);
                let re = self.builder.inst_results(inst);

                let value = re[0];
                let is_error = re[1];

                self.handle_error(is_error, value);
                self.store_reg(result, value);

                self.builder.ins().jump(exit, &[]);

                self.builder.seal_block(exit);
                self.builder.switch_to_block(exit);
            },
            OpCode::EqEqEq { result, left, right } => {
                todo!()
            },
            OpCode::LtEq { result, left, right } => {
                self.binary(left, right, result, operations::lteq, 
                    |this, lhs, rhs|{
                        let l = this.builder.ins().bitcast(types::F64, lhs);
                        let r = this.builder.ins().bitcast(types::F64, rhs);

                        let b = this.builder.ins().fcmp(ir::condcodes::FloatCC::LessThanOrEqual, l, r);
                        let t = this.builder.ins().iconst(JVALUE_TYPE, JValue::TRUE_TAG as i64);
                        let f = this.builder.ins().iconst(JVALUE_TYPE, JValue::FALSE_TAG as i64);
                        this.builder.ins().select(b, t, f)
                    }, 
                    |this, lhs, rhs|{
                        let b = this.builder.ins().icmp(ir::condcodes::IntCC::SignedLessThanOrEqual, lhs, rhs);
                        let t = this.builder.ins().iconst(JVALUE_TYPE, JValue::TRUE_TAG as i64);
                        let f = this.builder.ins().iconst(JVALUE_TYPE, JValue::FALSE_TAG as i64);
                        this.builder.ins().select(b, t, f)
                    }
                );
            },
            
            /////////////////////////////////////////////////////////////////////////////////
            OpCode::LoadFalse { result } => {
                let value = self.builder.ins().iconst(JVALUE_TYPE, JValue::FALSE_TAG as i64);
                self.store_reg(result, value)
            },
            OpCode::LoadTrue { result } => {
                let value = self.builder.ins().iconst(JVALUE_TYPE, JValue::TRUE_TAG as i64);
                self.store_reg(result, value)
            },
            OpCode::LoadNull { result } => {
                let value = self.builder.ins().iconst(JVALUE_TYPE, JValue::NULL_TAG as i64);
                self.store_reg(result, value)
            },
            OpCode::LoadUndefined { result } => {
                let value = self.builder.ins().iconst(JVALUE_TYPE, JValue::UNDEFINED_TAG as i64);
                self.store_reg(result, value)
            },
            OpCode::LoadStaticFloat32 { result, value } => {
                let value = self.builder.ins().iconst(JVALUE_TYPE, JValue::create_number(value as f64).to_bits() as i64);
                self.store_reg(result, value)
            },
            code => todo!("{:?}", code)
        }
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
