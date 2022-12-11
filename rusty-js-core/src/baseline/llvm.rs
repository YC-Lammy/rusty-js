use std::collections::HashMap;

use inkwell::basic_block::BasicBlock;
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::debug_info::{DebugInfoBuilder, DICompileUnit, DIFile, AsDIScope, DISubprogram};
use inkwell::execution_engine::{ExecutionEngine, JitFunction};
use inkwell::module::Module;
use inkwell::types::{FunctionType, StructType, IntType, FloatType};
use inkwell::values::{FunctionValue, IntValue, PointerValue};
use inkwell::{AddressSpace};

use crate::bytecodes::{Block, OpCode, Register};
use crate::runtime::Profiler;
use crate::{operations, JValue, Runtime};

use super::ExitCode;
pub struct CodeGen<'ctx> {
    name: String,

    context: &'ctx Context,
    module: &'ctx Module<'ctx>,
    execution_engine: &'ctx ExecutionEngine<'ctx>,

    builder: Builder<'ctx>,

    debug_builder:Option<DebugInfoBuilder<'ctx>>,
    debug_compiler:Option<DICompileUnit<'ctx>>,
    debug_file:Option<DIFile<'ctx>>,
    debug_func:Option<DISubprogram<'ctx>>,

    main_body_block: BasicBlock<'ctx>,
    switch_block: BasicBlock<'ctx>,
    error_exit_block: BasicBlock<'ctx>,
    return_value: PointerValue<'ctx>,
    return_block: BasicBlock<'ctx>,

    binary_fn_type: FunctionType<'ctx>,
    binary_fn_result_ty:StructType<'ctx>,
    i64_ty:IntType<'ctx>,
    f64_ty:FloatType<'ctx>,

    func: FunctionValue<'ctx>,
    registers: [inkwell::values::PointerValue<'ctx>; 3],
    
    functions:HashMap<&'static str, FunctionValue<'ctx>>,

    pub speculate_offset: usize,

    blocks: HashMap<Block, BasicBlock<'ctx>>,
    stack_alloc:HashMap<u16, PointerValue<'ctx>>,
    temps:Vec<PointerValue<'ctx>>,

    yield_value_ptr: PointerValue<'ctx>,
    breaking_blocks: Vec<BasicBlock<'ctx>>,

    catch_value: PointerValue<'ctx>,
    catch_blocks: Vec<BasicBlock<'ctx>>,

    for_in_iterators: Vec<PointerValue<'ctx>>,
    for_of_iterators: Vec<PointerValue<'ctx>>,
    iterator_done: PointerValue<'ctx>
}

pub type JSJITFunction = unsafe extern "C" fn(
    JValue,
    *const Runtime,
    *const JValue,
    usize,
    *mut JValue,
    *mut JValue,
    *mut JValue,
    *mut u32,
    JValue,
    JValue,
    JValue,
    JValue,
    *mut u16
) -> [u64; 5];

impl<'ctx> CodeGen<'ctx> {
    pub fn new(
        context: &'ctx Context,
        module: &'ctx Module<'ctx>,
        engine: &'ctx ExecutionEngine<'ctx>,
    ) -> Self {
        let i64_type = context.i64_type();
        let i64_array_type = i64_type.array_type(5);

        let f64_type = context.f64_type();

        /*fn(
            this: JValue,
            rt: &Runtime,
            args: *const JValue,
            argc: usize,
            stack: *mut JValue,
            op_stack: *mut JValue,
            capture_stack: *mut JValue,
            async_counter: *mut u32,
            yield_value: JValue,
            r0: JValue,
            r1: JValue,
            r2: JValue,
            profiler: *mut u16
        ) -> [u64;5]
        */
        let fn_type = i64_array_type.fn_type(
            &[
                i64_type.into(),
                i64_type.ptr_type(AddressSpace::Generic).into(),
                i64_type.ptr_type(AddressSpace::Generic).into(),
                #[cfg(target_pointer_width = "64")]
                i64_type.into(),
                #[cfg(target_pointer_width = "32")]
                context.i32_type().into(),
                i64_type.ptr_type(AddressSpace::Generic).into(),
                i64_type.ptr_type(AddressSpace::Generic).into(),
                i64_type.ptr_type(AddressSpace::Generic).into(),
                context.i32_type().ptr_type(AddressSpace::Generic).into(),
                i64_type.into(),
                i64_type.into(),
                i64_type.into(),
                i64_type.into(),
                context.i16_type().ptr_type(AddressSpace::Generic).into()
            ],
            false,
        );

        let binary_result_ty = context.struct_type(&[i64_type.into(), context.bool_type().into()], false);

        // fn(JValue, JValue, *mut JValue, &Runtime, &mut Result)
        let binary_fn_type = context.void_type()
            .fn_type(
                &[
                    i64_type.into(),
                    i64_type.into(),
                    i64_type.ptr_type(AddressSpace::Generic).into(),
                    i64_type.ptr_type(AddressSpace::Generic).into(),
                    binary_result_ty.ptr_type(AddressSpace::Generic).into()
                ],
                false,
            );

        let builder = context.create_builder();
        
        let name = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
            .to_string();
        let func = module.add_function(&name, fn_type, None);
        
        let (debug_builder, debug_compile, debug_file, debug_func) = if false{
            let (debug_builder, debug_compile) = module.create_debug_info_builder(
                true, 
                inkwell::debug_info::DWARFSourceLanguage::C, 
                "JS\0", 
                "JIT\0", 
                "YC\0", 
                false, 
                "", 
                0, 
                "", 
                inkwell::debug_info::DWARFEmissionKind::Full, 
                0, 
                false, 
                false, 
                "", 
                ""
            );
            
            let debug_file = debug_builder.create_file("jit", &name);
            let sub = debug_builder.create_subroutine_type(debug_file, None, &[], 0);

            let debug_func = debug_builder.create_function(
                debug_file.as_debug_info_scope(),
                &name,
                None,
                debug_file,
                0,
                sub,
                false,
                true,
                0,
                0,
                false
            );
            (Some(debug_builder), Some(debug_compile), Some(debug_file), Some(debug_func))
        } else{
            (None, None, None, None)
        };
        

        let basic_block = context.append_basic_block(func, "entry\0");

        builder.position_at_end(basic_block);

        let yield_value = builder.build_alloca(i64_type, "yield_value\0");
        let catch_value = builder.build_alloca(i64_type, "catch_value\0");
        let return_value = builder.build_alloca(i64_type, "return_value\0");
        let iter_done = builder.build_alloca(context.bool_type(), "iter_done\0");

        let registers = [
            builder.build_alloca(i64_type, "r0\0"),
            builder.build_alloca(i64_type, "r1\0"),
            builder.build_alloca(i64_type, "r2\0"),
        ];

        builder.build_store(yield_value, func.get_nth_param(8).unwrap());

        builder.build_store(registers[0], func.get_nth_param(9).unwrap());
        builder.build_store(registers[1], func.get_nth_param(10).unwrap());
        builder.build_store(registers[2], func.get_nth_param(11).unwrap());

        let switch_block = context.append_basic_block(func, "switch_block\0");

        builder.build_unconditional_branch(switch_block);

        let error_exit_block = context.append_basic_block(func, "error_exit\0");
        builder.position_at_end(error_exit_block);

        let undefined = i64_type.const_int(JValue::UNDEFINED_TAG, false);
        let error = builder.build_load(catch_value, "load_error\0");
        let exit_code = i64_type.const_int(ExitCode::Error as u64, false);

        builder.build_aggregate_return(&[
            error.into(),
            undefined.into(),
            undefined.into(),
            undefined.into(),
            exit_code.into(),
        ]);

        let normal_return_block = context.append_basic_block(func, "return_block\0");
        builder.position_at_end(normal_return_block);

        let undefined = i64_type.const_int(JValue::UNDEFINED_TAG, false);
        let value = builder.build_load(return_value, "load_return\0");
        let exit_code = i64_type.const_int(ExitCode::Return as u64, false);

        builder.build_aggregate_return(&[
            value.into(),
            undefined.into(),
            undefined.into(),
            undefined.into(),
            exit_code.into(),
        ]);

        let main_body = context.append_basic_block(func, "body\0");

        builder.position_at_end(main_body);

        Self {
            name: name,
            context: context,
            module: module,
            execution_engine: engine,
            builder: builder,
            debug_builder:debug_builder,
            debug_compiler:debug_compile,
            debug_file:debug_file,
            debug_func:debug_func,

            error_exit_block: error_exit_block,
            switch_block: switch_block,
            return_block: normal_return_block,
            return_value: return_value,
            main_body_block: main_body,

            binary_fn_type: binary_fn_type,
            binary_fn_result_ty:binary_result_ty,
            i64_ty:i64_type,
            f64_ty: f64_type,

            functions:Default::default(),

            func: func,
            registers: registers,

            blocks: Default::default(),
            stack_alloc: Default::default(),
            speculate_offset:0,
            temps: Default::default(),

            yield_value_ptr: yield_value,
            breaking_blocks: Vec::new(),

            catch_value: catch_value,
            catch_blocks: Vec::new(),

            for_in_iterators: Default::default(),
            for_of_iterators: Default::default(),
            iterator_done:  iter_done,
        }
    }

    pub fn translate_codes(&mut self, codes: &[OpCode]) -> super::Function {
        for code in codes {
            let code = *code;
            self.translate_code(code);
        }

        self.builder.position_at_end(self.switch_block);

        let done_block = self.context.append_basic_block(self.func, "done_b");
        let counter_ptr = self.func.get_nth_param(7).unwrap().into_pointer_value();
        let count = self
            .builder
            .build_load(counter_ptr, "load_counter_value\0")
            .into_int_value();

        let mut entries = vec![(
            self.context.i32_type().const_int(0, false),
            self.main_body_block,
        )];

        for i in 0..self.breaking_blocks.len() {
            entries.push((
                self.context.i32_type().const_int(i as u64 + 1, false),
                self.breaking_blocks[i],
            ));
        }

        self.builder.build_switch(count, done_block, &entries);

        self.builder.position_at_end(done_block);

        let undefined = self
            .i64_ty
            .const_int(JValue::UNDEFINED_TAG, false);
        let exit_code = self
            .i64_ty
            .const_int(ExitCode::Done as u64, false);

        self.builder.build_aggregate_return(&[
            undefined.into(),
            undefined.into(),
            undefined.into(),
            undefined.into(),
            exit_code.into(),
        ]);

        let size = self.speculate_offset;

        let f:JitFunction<JSJITFunction> = unsafe { self.execution_engine.get_function(&self.name).unwrap() };
        let f = unsafe{std::mem::transmute(f)};

        super::Function{
            profiler:Profiler::new(size),
            function:f
        }
    }

    fn speculate(&mut self, values: &[IntValue<'ctx>]) {
        let profiler = self.func.get_nth_param(12).unwrap().into_pointer_value();
        let profiler = self.builder.build_ptr_to_int(profiler, self.i64_ty, "\0");

        for i in values{
            let offset = self.i64_ty.const_int(self.speculate_offset as u64 * 2, false);
            let ptr = self.builder.build_int_add(profiler, offset, "add\0");
            let ptr = self.builder.build_int_to_ptr(ptr, self.context.i16_type().ptr_type(AddressSpace::Generic), "int_to_ptr\0");

            let shift = self.i64_ty.const_int(48, false);
            let flag = self.builder.build_right_shift(*i, shift, false, "shift\0");
            let flag = self.builder.build_int_truncate(flag, self.context.i16_type(), "trunc\0");
            
            let old = self.builder.build_load(ptr, "load\0").into_int_value();
            let new = self.builder.build_or(flag, old, "or\0");

            self.builder.build_store(ptr, new);

            self.speculate_offset += 1;
        }
    }

    fn read_reg(&mut self, r: Register) -> inkwell::values::IntValue<'ctx> {
        let ptr = self.registers[r.0 as usize];
        self.builder.build_load(ptr, "read_reg\0").into_int_value()
    }

    fn store_reg(&mut self, r: Register, value: inkwell::values::IntValue<'ctx>) {
        let ptr = self.registers[r.0 as usize];
        self.builder.build_store(ptr, value);
    }

    fn to_bool(&mut self, value: IntValue<'ctx>) -> IntValue<'ctx> {
        let b = self.i64_ty.const_int(16, false);
        let b = self.builder.build_left_shift(value, b, "to_bool\0");
        let zero = self.i64_ty.const_zero();
        self.builder
            .build_int_compare(inkwell::IntPredicate::NE, b, zero, "ne\0")
    }

    fn is_float(&mut self, value: IntValue<'ctx>) -> IntValue<'ctx> {
        let r = self.i64_ty.const_int(JValue::NAN_BITS, false);
        let c = self.builder.build_and(value, r, "\0");
        self.builder
            .build_int_compare(inkwell::IntPredicate::NE, c, r, "\0")
    }

    fn is_object(&mut self, value: IntValue<'ctx>) -> IntValue<'ctx> {
        let b = self.i64_ty.const_int(48, false);
        let tag = self.builder.build_right_shift(value, b, false, "rshift\0");
        let obj_tag = self
            .i64_ty
            .const_int(JValue::OBJECT_TAG >> 48, false);
        self.builder.build_and(tag, obj_tag, "is_object\0")
    }

    fn handle_error(&mut self, value: IntValue<'ctx>, is_error: IntValue<'ctx>){
        let exit = self.context.append_basic_block(self.func, "exit\0");

        self.builder.build_store(self.catch_value, value);

        if let Some(catch) = self.catch_blocks.last(){
            let catch = *catch;
            self.builder.build_conditional_branch(is_error, catch, exit);
        } else{
            self.builder.build_conditional_branch(is_error, self.error_exit_block, exit);
        };

        self.builder.position_at_end(exit);
    }

    fn binary<F0>(
        &mut self,
        left: Register,
        right: Register,
        result: Register,
        slow_fn: extern "C" fn(JValue, JValue, *mut JValue, &Runtime, &mut operations::Result),
        name: &'static str,
        f_body: F0,
    ) -> ()
    where
        F0: Fn(
            &mut Self,
            inkwell::values::IntValue<'ctx>,
            inkwell::values::IntValue<'ctx>,
        ) -> inkwell::values::IntValue<'ctx>,
    {
        let fast_path = self.context.append_basic_block(self.func, "exit\0");
        let slow_path = self.context.append_basic_block(self.func, "fastpath\0");
        let exit = self.context.append_basic_block(self.func, "slowpath\0");

        let lhs = self.read_reg(left);
        let rhs = self.read_reg(right);

        self.speculate(&[lhs, rhs]);

        let l_is_float = self.is_float(lhs);
        let r_is_float = self.is_float(rhs);

        let is_float = self.builder.build_and(l_is_float, r_is_float, "isfloat\0");

        self.builder
            .build_conditional_branch(is_float, fast_path, slow_path);

        // fast path
        self.builder.position_at_end(fast_path);

        let re = f_body(self, lhs, rhs);

        self.store_reg(result, re);

        self.builder.build_unconditional_branch(exit);

        // slow path
        self.builder.position_at_end(slow_path);

        let func = match self.functions.get(&name){
            Some(f) => *f,
            None => {
                match self.module.get_function(name) {
                    Some(f) => {
                        self.functions.insert(name, f);
                        f
                    },
                    None => {
                        let f = self.module.add_function(name, self.binary_fn_type, None);
                        self.execution_engine
                            .add_global_mapping(&f, slow_fn as usize);
                        self.functions.insert(name, f);
                        f
                    }
                }
            }
        };

        // get op_stack
        let stack = self.func.get_nth_param(5).unwrap().into_pointer_value();
        // get runtime
        let runtime = self.func.get_nth_param(1).unwrap().into_pointer_value();

        let ret_ptr = self.builder.build_alloca(self.binary_fn_result_ty, "call result");

        self.builder.build_call(
            func,
            &[lhs.into(), rhs.into(), stack.into(), runtime.into(), ret_ptr.into()],
            "slow_path\0",
        );

        let ret = self.builder.build_load(ret_ptr, "load_result\0").into_struct_value();

        let value = self
            .builder
            .build_extract_value(ret, 0, "extract\0")
            .unwrap()
            .into_int_value();
        let is_error = self
            .builder
            .build_extract_value(ret, 1, "extract\0")
            .unwrap()
            .into_int_value();

        self.store_reg(result, value);
        self.builder.build_store(self.catch_value, value);

        if let Some(catch) = self.catch_blocks.last() {
            let catch = *catch;
            self.builder.build_conditional_branch(is_error, catch, exit);
        } else {
            self.builder
                .build_conditional_branch(is_error, self.error_exit_block, exit);
        };

        self.builder.position_at_end(exit);
    }

    /// helper function to create operations for immediate binary instructions
    fn binary_imm<F0>(
        &mut self,
        left: Register,
        right: f64,
        result: Register,
        slow_fn: extern "C" fn(JValue, JValue, *mut JValue, &Runtime, &mut operations::Result),
        name:&'static str,
        f_body: F0,
    ) -> ()
    where
        F0: Fn(&mut Self, inkwell::values::IntValue<'ctx>, f64) -> inkwell::values::IntValue<'ctx>,
    {
        let fast_path = self.context.append_basic_block(self.func, "exit\0");
        let slow_path = self.context.append_basic_block(self.func, "fastpath\0");
        let exit = self.context.append_basic_block(self.func, "slowpath\0");

        let lhs = self.read_reg(left);

        self.speculate(&[lhs]);

        let is_float = self.is_float(lhs);

        self.builder
            .build_conditional_branch(is_float, fast_path, slow_path);

        // fast path
        self.builder.position_at_end(fast_path);

        let re = f_body(self, lhs, right);

        self.store_reg(result, re);

        self.builder.build_unconditional_branch(exit);

        // slow path
        self.builder.position_at_end(slow_path);

        let func = match self.functions.get(&name){
            Some(f) => *f,
            None => {
                match self.module.get_function(name) {
                    Some(f) => {
                        self.functions.insert(name, f);
                        f
                    },
                    None => {
                        let f = self.module.add_function(name, self.binary_fn_type, None);
                        self.execution_engine
                            .add_global_mapping(&f, slow_fn as usize);
                        self.functions.insert(name, f);
                        f
                    }
                }
            }
        };

        let stack = self.func.get_nth_param(5).unwrap().into_pointer_value();
        let runtime = self.func.get_nth_param(1).unwrap().into_pointer_value();

        let rhs = self.i64_ty.const_int(JValue::create_number(right).to_bits(), false);

        let ret_ptr = self.builder.build_alloca(self.binary_fn_result_ty, "call_result\0");

        self.builder.build_call(
            func,
            &[lhs.into(), rhs.into(), stack.into(), runtime.into(), ret_ptr.into()],
            "slow_path\0",
        );

        let ret = self.builder.build_load(ret_ptr, "load_result\0").into_struct_value();

        let value = self
            .builder
            .build_extract_value(ret, 0, "extract\0")
            .unwrap()
            .into_int_value();
        let is_error = self
            .builder
            .build_extract_value(ret, 1, "extract\0")
            .unwrap()
            .into_int_value();

        self.store_reg(result, value);
        self.builder.build_store(self.catch_value, value);

        if let Some(catch) = self.catch_blocks.last() {
            let catch = *catch;
            self.builder.build_conditional_branch(is_error, catch, exit);
        } else {
            self.builder
                .build_conditional_branch(is_error, self.error_exit_block, exit);
        };

        self.builder.position_at_end(exit);
    }

    fn translate_code(&mut self, code: OpCode) {
        match code {
            OpCode::NoOp => {}
            OpCode::Debugger => {}

            OpCode::Mov { from, to } => {
                let v = self.read_reg(from);
                self.store_reg(to, v);
            }

            OpCode::Return { value } => {
                let value = self.read_reg(value);
                self.builder.build_store(self.return_value, value);
                self.builder.build_unconditional_branch(self.return_block);
            }
            OpCode::Throw { value } => {
                if let Some(catch) = self.catch_blocks.last() {
                    let catch = *catch;
                    let value = self.read_reg(value);
                    self.builder.build_store(self.catch_value, value);

                    self.builder.build_unconditional_branch(catch);
                } else {
                    let value = self.read_reg(value);
                    let undefined = self
                        .i64_ty
                        .const_int(JValue::UNDEFINED_TAG, false);
                    let exit_code = self
                        .i64_ty
                        .const_int(ExitCode::Error as u64, false);

                    self.builder.build_aggregate_return(&[
                        value.into(),
                        undefined.into(),
                        undefined.into(),
                        undefined.into(),
                        exit_code.into(),
                    ]);
                }
            }

            OpCode::Await { result, future } => {
                let resume_block = self.context.append_basic_block(self.func, "await_block\0");
                let break_block = self
                    .context
                    .append_basic_block(self.func, "await_breaking_block\0");

                // push the block for building jump table
                self.breaking_blocks.push(resume_block);

                let counter = self.breaking_blocks.len();
                let counter = self.context.i32_type().const_int(counter as u64, false);

                // store the counter for resuming
                let counter_ptr = self.func.get_nth_param(7).unwrap().into_pointer_value();
                self.builder.build_store(counter_ptr, counter);

                let future = self.read_reg(future);

                let is_obj = self.is_object(future);

                // store the future to yield for resume block
                self.builder.build_store(self.yield_value_ptr, future);

                // jump to resume block directly if not object
                self.builder
                    .build_conditional_branch(is_obj, break_block, resume_block);

                // breaking block
                self.builder.position_at_end(break_block);

                let exit_code = self
                    .i64_ty
                    .const_int(ExitCode::Await as u64, false);
                let r0 = self.read_reg(Register(0));
                let r1 = self.read_reg(Register(1));
                let r2 = self.read_reg(Register(2));

                // return the future with await exit code
                self.builder.build_aggregate_return(&[
                    future.into(),
                    r0.into(),
                    r1.into(),
                    r2.into(),
                    exit_code.into(),
                ]);

                // resume block
                self.builder.position_at_end(resume_block);

                let value = self
                    .builder
                    .build_load(self.yield_value_ptr, "load_yield\0")
                    .into_int_value();
                self.store_reg(result, value);
            },
            OpCode::Yield { result, arg } => {
                let resume_block = self.context.append_basic_block(self.func, "await_block\0");
                let break_block = self
                    .context
                    .append_basic_block(self.func, "await_breaking_block\0");

                // push the block for building jump table
                self.breaking_blocks.push(resume_block);

                let counter = self.breaking_blocks.len();
                let counter = self.context.i32_type().const_int(counter as u64, false);

                // store the counter for resuming
                let counter_ptr = self.func.get_nth_param(7).unwrap().into_pointer_value();
                self.builder.build_store(counter_ptr, counter);

                let future = self.read_reg(arg);

                // jump to the breaking block
                self.builder.build_unconditional_branch(break_block);

                // breaking block
                self.builder.position_at_end(break_block);

                let exit_code = self
                    .i64_ty
                    .const_int(ExitCode::Yield as u64, false);
                let r0 = self.read_reg(Register(0));
                let r1 = self.read_reg(Register(1));
                let r2 = self.read_reg(Register(2));

                // return the future with await exit code
                self.builder.build_aggregate_return(&[
                    future.into(),
                    r0.into(),
                    r1.into(),
                    r2.into(),
                    exit_code.into(),
                ]);

                // resume block
                self.builder.position_at_end(resume_block);

                let value = self
                    .builder
                    .build_load(self.yield_value_ptr, "load_yield\0")
                    .into_int_value();
                self.store_reg(result, value);
            }

            /////////////////////////////////////////////////////////////
            //          Block
            /////////////////////////////////////////////////////////////
            OpCode::CreateBlock(b) => {
                let block = self.context.append_basic_block(self.func, "block\0");
                self.blocks.insert(b, block);
            }
            OpCode::SwitchToBlock(b) => {
                self.builder.position_at_end(self.blocks[&b]);
            }
            OpCode::Jump { to, line: _ } => {
                self.builder.build_unconditional_branch(self.blocks[&to]);
            }
            OpCode::JumpIfTrue { value, to, line: _ } => {
                let else_block = self.context.append_basic_block(self.func, "else_block\0");

                let value = self.read_reg(value);
                let is_true = self.to_bool(value);

                self.builder
                    .build_conditional_branch(is_true, self.blocks[&to], else_block);

                self.builder.position_at_end(else_block);
            }
            OpCode::JumpIfFalse { value, to, line: _ } => {
                let else_block = self.context.append_basic_block(self.func, "else_block\0");

                let value = self.read_reg(value);
                let is_true = self.to_bool(value);

                let is_false = self.builder.build_not(is_true, "not_is_true\0");

                self.builder
                    .build_conditional_branch(is_false, self.blocks[&to], else_block);

                self.builder.position_at_end(else_block);
            }
            OpCode::EnterTry { catch_block, line:_ } => {
                let catch = self.context.append_basic_block(self.func, "catch_block\0");
                self.blocks.insert(catch_block, catch);

                self.catch_blocks.push(catch);
            },

            OpCode::ExitTry => {
                self.catch_blocks.pop();
            }

            /////////////////////////////////////////////////////
            //            Memory
            ////////////////////////////////////////////////////
            OpCode::ReadParam { result, index } => {
                let ptr = self.func.get_nth_param(2).unwrap().into_pointer_value();
                let argc = self.func.get_nth_param(3).unwrap().into_int_value();

                let undefined = self
                    .i64_ty
                    .const_int(JValue::UNDEFINED_TAG, false);

                let ptr = self
                    .builder
                    .build_ptr_to_int(ptr, self.i64_ty, "ptr\0");
                let offset = self
                    .i64_ty
                    .const_int(index as u64 * std::mem::size_of::<JValue>() as u64, false);
                let ptr = self.builder.build_int_add(ptr, offset, "ptr\0");
                let ptr = self.builder.build_int_to_ptr(
                    ptr,
                    self.i64_ty.ptr_type(AddressSpace::Generic),
                    "ptr\0",
                );

                let value = self.builder.build_load(ptr, "load_param").into_int_value();

                let index = self.i64_ty.const_int(index as u64, false);

                // if argc is greater than index, use the loaded value else, undefined
                let lt = self.builder.build_int_compare(
                    inkwell::IntPredicate::UGT,
                    argc,
                    index,
                    "index\0",
                );
                let value = self
                    .builder
                    .build_select(lt, value, undefined, "select\0")
                    .into_int_value();

                self.store_reg(result, value);
            }
            OpCode::CollectParam { result, start } => {
                todo!()
            }
            OpCode::ReadFromStack {
                result,
                stack_offset,
            } => {
                /*
                let value = self.builder.build_load(self.stack_alloc[&stack_offset], "load_from_stack\0").into_int_value();
                self.store_reg(result, value);
                */

                
                let stack = self.func.get_nth_param(4).unwrap().into_pointer_value();
                let offset = self.i64_ty.const_int(
                    stack_offset as u64 * std::mem::size_of::<JValue>() as u64,
                    false,
                );
                let ptr_ty = self.i64_ty.ptr_type(AddressSpace::Generic);

                let stack =
                    self.builder
                        .build_ptr_to_int(stack, self.i64_ty, "stack\0");
                let stack = self
                    .builder
                    .build_int_add(stack, offset, "stack_add_offset\0");
                let stack = self
                    .builder
                    .build_int_to_ptr(stack, ptr_ty, "stack_pointer\0");
                let value = self
                    .builder
                    .build_load(stack, "load_from_stack\0")
                    .into_int_value();
                self.store_reg(result, value);
            }
            OpCode::WriteToStack { from, stack_offset } => {
                
                let stack = self.func.get_nth_param(4).unwrap().into_pointer_value();
                let offset = self.i64_ty.const_int(
                    stack_offset as u64 * std::mem::size_of::<JValue>() as u64,
                    false,
                );
                let ptr_ty = self.i64_ty.ptr_type(AddressSpace::Generic);

                let stack =
                    self.builder
                        .build_ptr_to_int(stack, self.i64_ty, "stack\0");
                let stack = self
                    .builder
                    .build_int_add(stack, offset, "stack_add_offset\0");
                let stack = self
                    .builder
                    .build_int_to_ptr(stack, ptr_ty, "stack_pointer\0");
                let value = self.read_reg(from);

                self.builder.build_store(stack, value);
                
            }
            OpCode::StoreTemp { value } => {
                let temp = self.builder.build_alloca(self.i64_ty, "temp\0");
                let v = self.read_reg(value);
                self.builder.build_store(temp, v);
                self.temps.push(temp);
            }
            OpCode::ReadTemp { value } => {
                let temp = self.temps[self.temps.len() - 1];
                let v = self.builder.build_load(temp, "temp_read\0").into_int_value();
                self.store_reg(value, v);
            }
            OpCode::ReleaseTemp => {
                self.temps.pop();
            }
            OpCode::LoadThis { result } => {
                let this = self.func.get_nth_param(0).unwrap().into_int_value();
                self.store_reg(result, this);
            }
            OpCode::LoadFalse { result } => {
                let value = self.i64_ty.const_int(JValue::FALSE_TAG, false);
                self.store_reg(result, value);
            }
            OpCode::LoadTrue { result } => {
                let value = self.i64_ty.const_int(JValue::TRUE_TAG, false);
                self.store_reg(result, value);
            }
            OpCode::LoadUndefined { result } => {
                let value = self
                    .i64_ty
                    .const_int(JValue::UNDEFINED_TAG, false);
                self.store_reg(result, value);
            },
            OpCode::LoadNull { result } => {
                let value = self
                    .i64_ty
                    .const_int(JValue::NULL_TAG, false);
                self.store_reg(result, value);
            }
            OpCode::LoadStaticFloat32 { result, value } => {
                let value = self
                    .i64_ty
                    .const_int(JValue::create_number(value as f64).to_bits(), false);
                self.store_reg(result, value);
            }

            OpCode::DeclareDynamicVar { from, kind, offset } => {

            },
            OpCode::WriteDynamicVarDeclared { from, offset } => {

            },
            OpCode::ReadDynamicVarDeclared { result, offset } => {

            },
            OpCode::WriteDynamicVar { from, id } => {

            },
            OpCode::ReadDynamicVar { result, id } => {

            }
            OpCode::ReadCapturedVar { result, offset } => {

            }
            OpCode::WriteCapturedVar { from, offset } => {

            }

            //

            OpCode::Select { a, b, result } => {
                let lhs = self.read_reg(a);
                let rhs = self.i64_ty.const_int(JValue::UNDEFINED_TAG, false);
                let is_undefined = self.builder.build_int_compare(inkwell::IntPredicate::EQ, lhs, rhs, "select\0");
                let b = self.read_reg(b);
                let value = self.builder.build_select(is_undefined, b, lhs, "selct\0").into_int_value();
                self.store_reg(result, value)
            }
            OpCode::CondSelect { t, a, b, result } => {
                let t = self.read_reg(t);
                let a = self.read_reg(a);
                let b = self.read_reg(b);
                let is_true = self.to_bool(t);
                let value = self.builder.build_select(is_true, a, b, "condselect\0").into_int_value();
                self.store_reg(result, value);
            }

            OpCode::CreateArg { stack_offset, len } => {

            }
            OpCode::PushArg { value, stack_offset } => {
                let stack = self.func.get_nth_param(4).unwrap().into_pointer_value();
                let offset = self.i64_ty.const_int(
                    stack_offset as u64 * std::mem::size_of::<JValue>() as u64,
                    false,
                );
                let ptr_ty = self.i64_ty.ptr_type(AddressSpace::Generic);

                let stack =
                    self.builder
                        .build_ptr_to_int(stack, self.i64_ty, "stack\0");
                let stack = self
                    .builder
                    .build_int_add(stack, offset, "stack_add_offset\0");
                let stack = self
                    .builder
                    .build_int_to_ptr(stack, ptr_ty, "stack_pointer\0");
                let value = self.read_reg(value);

                self.builder.build_store(stack, value);
            }
            OpCode::PushArgSpread { value, stack_offset } => {
                todo!()
            }
            OpCode::SpreadArg { base_stack_offset, stack_offset, args_len } => {
                todo!()
            }
            OpCode::FinishArgs { base_stack_offset, len } => {

            }
            OpCode::Call { result, this, callee, stack_offset, args_len } => {

                let this = self.read_reg(this);
                let callee = self.read_reg(callee);
                let stack_offset = self.i64_ty.const_int(stack_offset as u64 * std::mem::size_of::<JValue>() as u64, false);

                #[cfg(target_pointer_width = "64")]
                let argc = self.i64_ty.const_int(args_len as u64, false);

                #[cfg(target_pointer_width = "32")]
                let argc = self.context.i32_type().const_int(args_len as u64, false);

                let ptr_ty = self.i64_ty.ptr_type(AddressSpace::Generic);

                let runtime = self.func.get_nth_param(1).unwrap().into_pointer_value();

                let stack = self.func.get_nth_param(4).unwrap().into_pointer_value();
                let stack = self.builder
                        .build_ptr_to_int(stack, self.i64_ty, "stack\0");
                let stack = self
                    .builder
                    .build_int_add(stack, stack_offset, "stack_add_offset\0");

                let stack = self
                    .builder
                    .build_int_to_ptr(stack, ptr_ty, "stack_pointer\0");

                let func = match self.functions.get("call"){
                    Some(f) => *f,
                    None => {
                        let ty = self.context.void_type().fn_type(
                            &[
                                self.i64_ty.into(),
                                self.i64_ty.ptr_type(AddressSpace::Generic).into(),
                                self.i64_ty.into(),
                                self.i64_ty.ptr_type(AddressSpace::Generic).into(),
                                #[cfg(target_pointer_width = "64")]
                                self.i64_ty.into(),
                                #[cfg(target_pointer_width = "32")]
                                self.context.i32_type().into(),
                                self.binary_fn_result_ty.ptr_type(AddressSpace::Generic).into()
                            ], 
                            false
                        );
                        let func = self.module.add_function("call", ty, None);
                        self.functions.insert("call", func);
                        self.execution_engine.add_global_mapping(&func, operations::call as usize);
                        func
                    }
                };

                let exit = self.context.append_basic_block(self.func, "exit\0");

                let re = self.builder.build_alloca(self.binary_fn_result_ty, "result\0");

                self.builder.build_call(
                    func, 
                    &[
                        callee.into(),
                        runtime.into(),
                        this.into(),
                        stack.into(),
                        argc.into(),
                        re.into()
                    ], 
                    "call\0"
                );

                let ret = self.builder.build_load(re, "load_result\0").into_struct_value();

                let value = self
                    .builder
                    .build_extract_value(ret, 0, "extract\0")
                    .unwrap()
                    .into_int_value();

                let is_error = self
                    .builder
                    .build_extract_value(ret, 1, "extract\0")
                    .unwrap()
                    .into_int_value();

                self.store_reg(result, value);
                self.builder.build_store(self.catch_value, value);

                if let Some(catch) = self.catch_blocks.last() {
                    let catch = *catch;
                    self.builder.build_conditional_branch(is_error, catch, exit);
                } else {
                    self.builder
                        .build_conditional_branch(is_error, self.error_exit_block, exit);
                };

                self.builder.position_at_end(exit);
            }
            OpCode::New { result, callee, stack_offset, args_len } => {

                let callee = self.read_reg(callee);
                let stack_offset = self.i64_ty.const_int(stack_offset as u64 * std::mem::size_of::<JValue>() as u64, false);

                #[cfg(target_pointer_width = "64")]
                let argc = self.i64_ty.const_int(args_len as u64, false);

                #[cfg(target_pointer_width = "32")]
                let argc = self.context.i32_type().const_int(args_len as u64, false);

                let ptr_ty = self.i64_ty.ptr_type(AddressSpace::Generic);

                let runtime = self.func.get_nth_param(1).unwrap().into_pointer_value();

                let stack = self.func.get_nth_param(4).unwrap().into_pointer_value();
                let stack = self.builder
                        .build_ptr_to_int(stack, self.i64_ty, "stack\0");
                let stack = self
                    .builder
                    .build_int_add(stack, stack_offset, "stack_add_offset\0");

                let stack = self
                    .builder
                    .build_int_to_ptr(stack, ptr_ty, "stack_pointer\0");

                let func = match self.functions.get("new"){
                    Some(f) => *f,
                    None => {
                        let ty = self.context.void_type().fn_type(
                            &[
                                self.i64_ty.into(),
                                self.i64_ty.ptr_type(AddressSpace::Generic).into(),
                                self.i64_ty.ptr_type(AddressSpace::Generic).into(),
                                #[cfg(target_pointer_width = "64")]
                                self.i64_ty.into(),
                                #[cfg(target_pointer_width = "32")]
                                self.context.i32_type().into(),
                                self.binary_fn_result_ty.ptr_type(AddressSpace::Generic).into()
                            ], 
                            false
                        );
                        let func = self.module.add_function("new", ty, None);
                        self.functions.insert("new", func);
                        self.execution_engine.add_global_mapping(&func, operations::invoke_new as usize);
                        func
                    }
                };

                let exit = self.context.append_basic_block(self.func, "exit\0");

                let re = self.builder.build_alloca(self.binary_fn_result_ty, "result\0");

                self.builder.build_call(
                    func, 
                    &[
                        callee.into(),
                        runtime.into(),
                        stack.into(),
                        argc.into(),
                        re.into()
                    ], 
                    "new\0"
                );

                let ret = self.builder.build_load(re, "load_result\0").into_struct_value();

                let value = self
                    .builder
                    .build_extract_value(ret, 0, "extract\0")
                    .unwrap()
                    .into_int_value();

                let is_error = self
                    .builder
                    .build_extract_value(ret, 1, "extract\0")
                    .unwrap()
                    .into_int_value();

                self.store_reg(result, value);
                self.builder.build_store(self.catch_value, value);

                if let Some(catch) = self.catch_blocks.last() {
                    let catch = *catch;
                    self.builder.build_conditional_branch(is_error, catch, exit);
                } else {
                    self.builder
                        .build_conditional_branch(is_error, self.error_exit_block, exit);
                };

                self.builder.position_at_end(exit);
            }

            OpCode::NewTarget { result } => {
                let runtime = self.func.get_nth_param(1).unwrap().into_pointer_value();
                let fun = match self.functions.get("new.target"){
                    Some(v) => *v,
                    None => {
                        let ty = self.i64_ty.fn_type(&[
                            self.i64_ty.ptr_type(AddressSpace::Generic).into()
                        ], false);

                        let fun = self.module.add_function("new_target\0", ty, None);
                        self.execution_engine.add_global_mapping(&fun, operations::new_target as usize);
                        self.functions.insert("new_target", fun);
                        fun
                    }
                };
                
                let site = self.builder.build_call(fun, &[runtime.into()], "import_meta\0");
                let value = site.try_as_basic_value().left().unwrap().into_int_value();
                self.store_reg(result, value);
            }
            OpCode::ImportMeta { result } => {
                let runtime = self.func.get_nth_param(1).unwrap().into_pointer_value();
                let fun = match self.functions.get("import.meta"){
                    Some(v) => *v,
                    None => {
                        let ty = self.i64_ty.fn_type(&[
                            self.i64_ty.ptr_type(AddressSpace::Generic).into()
                        ], false);

                        let fun = self.module.add_function("import_meta\0", ty, None);
                        self.execution_engine.add_global_mapping(&fun, operations::import_meta as usize);
                        self.functions.insert("import.meta", fun);
                        fun
                    }
                };
                
                let site = self.builder.build_call(fun, &[runtime.into()], "import_meta\0");
                let value = site.try_as_basic_value().left().unwrap().into_int_value();
                self.store_reg(result, value);
            }

            /////////////////////////////////////////////////////
            //            Binary
            ////////////////////////////////////////////////////
            OpCode::Add {
                result,
                left,
                right,
            } => {
                self.binary(
                    left,
                    right,
                    result,
                    operations::add,
                    "add",
                    |this, lhs, rhs| {
                        let l = this
                            .builder
                            .build_cast(
                                inkwell::values::InstructionOpcode::BitCast,
                                lhs,
                                this.f64_ty,
                                "cast\0",
                            )
                            .into_float_value();
                        let r = this
                            .builder
                            .build_cast(
                                inkwell::values::InstructionOpcode::BitCast,
                                rhs,
                                this.f64_ty,
                                "cast\0",
                            )
                            .into_float_value();

                        let f = this.builder.build_float_add(l, r, "add\0");
                        this.builder
                            .build_cast(
                                inkwell::values::InstructionOpcode::BitCast,
                                f,
                                this.i64_ty,
                                "cast\0",
                            )
                            .into_int_value()
                    },
                );
            }
            OpCode::AddImmI32 {
                result,
                left,
                right,
            } => {
                self.binary_imm(
                    left,
                    right as f64,
                    result,
                    operations::add,
                    "add",
                    |this, lhs, right| {
                        let l = this
                            .builder
                            .build_cast(
                                inkwell::values::InstructionOpcode::BitCast,
                                lhs,
                                this.f64_ty,
                                "cast\0",
                            )
                            .into_float_value();
                        let r = this.f64_ty.const_float(right);

                        let f = this.builder.build_float_add(l, r, "add\0");
                        this.builder
                            .build_cast(
                                inkwell::values::InstructionOpcode::BitCast,
                                f,
                                this.i64_ty,
                                "cast\0",
                            )
                            .into_int_value()
                    },
                );
            },
            OpCode::AddImmF32 { result, left, right } => {
                self.binary_imm(
                    left,
                    right as f64,
                    result,
                    operations::add,
                    "add",
                    |this, lhs, right| {
                        let l = this
                            .builder
                            .build_cast(
                                inkwell::values::InstructionOpcode::BitCast,
                                lhs,
                                this.f64_ty,
                                "cast\0",
                            )
                            .into_float_value();
                        let r = this.f64_ty.const_float(right);

                        let f = this.builder.build_float_add(l, r, "add\0");
                        this.builder
                            .build_cast(
                                inkwell::values::InstructionOpcode::BitCast,
                                f,
                                this.i64_ty,
                                "cast\0",
                            )
                            .into_int_value()
                    },
                );
            }
            OpCode::Sub {
                result,
                left,
                right,
            } => {
                self.binary(
                    left,
                    right,
                    result,
                    operations::sub,
                    "sub",
                    |this, lhs, rhs| {
                        let l = this
                            .builder
                            .build_cast(
                                inkwell::values::InstructionOpcode::BitCast,
                                lhs,
                                this.f64_ty,
                                "cast\0",
                            )
                            .into_float_value();
                        let r = this
                            .builder
                            .build_cast(
                                inkwell::values::InstructionOpcode::BitCast,
                                rhs,
                                this.f64_ty,
                                "cast\0",
                            )
                            .into_float_value();

                        let f = this.builder.build_float_sub(l, r, "sub\0");
                        this.builder
                            .build_cast(
                                inkwell::values::InstructionOpcode::BitCast,
                                f,
                                this.i64_ty,
                                "cast\0",
                            )
                            .into_int_value()
                    },
                );
            },
            OpCode::SubImmI32 { result, left, right } => {
                self.binary_imm(
                    left,
                    right as f64,
                    result,
                    operations::sub,
                    "sub",
                    |this, lhs, right| {
                        let l = this
                            .builder
                            .build_cast(
                                inkwell::values::InstructionOpcode::BitCast,
                                lhs,
                                this.f64_ty,
                                "cast\0",
                            )
                            .into_float_value();
                        let r = this.f64_ty.const_float(right);

                        let f = this.builder.build_float_sub(l, r, "sub\0");
                        this.builder
                            .build_cast(
                                inkwell::values::InstructionOpcode::BitCast,
                                f,
                                this.i64_ty,
                                "cast\0",
                            )
                            .into_int_value()
                    },
                );
            }
            OpCode::SubImmF32 { result, left, right } => {
                self.binary_imm(
                    left,
                    right as f64,
                    result,
                    operations::sub,
                    "sub",
                    |this, lhs, right| {
                        let l = this
                            .builder
                            .build_cast(
                                inkwell::values::InstructionOpcode::BitCast,
                                lhs,
                                this.f64_ty,
                                "cast\0",
                            )
                            .into_float_value();
                        let r = this.f64_ty.const_float(right);

                        let f = this.builder.build_float_sub(l, r, "sub\0");
                        this.builder
                            .build_cast(
                                inkwell::values::InstructionOpcode::BitCast,
                                f,
                                this.i64_ty,
                                "cast\0",
                            )
                            .into_int_value()
                    },
                );
            }
            OpCode::Mul {
                result,
                left,
                right,
            } => {
                self.binary(
                    left,
                    right,
                    result,
                    operations::mul,
                    "mul",
                    |this, lhs, rhs| {
                        let l = this
                            .builder
                            .build_cast(
                                inkwell::values::InstructionOpcode::BitCast,
                                lhs,
                                this.f64_ty,
                                "cast\0",
                            )
                            .into_float_value();
                        let r = this
                            .builder
                            .build_cast(
                                inkwell::values::InstructionOpcode::BitCast,
                                rhs,
                                this.f64_ty,
                                "cast\0",
                            )
                            .into_float_value();

                        let f = this.builder.build_float_mul(l, r, "mul\0");
                        this.builder
                            .build_cast(
                                inkwell::values::InstructionOpcode::BitCast,
                                f,
                                this.i64_ty,
                                "cast\0",
                            )
                            .into_int_value()
                    },
                );
            }
            OpCode::MulImmI32 { result, left, right } => {
                self.binary_imm(
                    left,
                    right as f64,
                    result,
                    operations::mul,
                    "mul",
                    |this, lhs, right| {
                        let l = this
                            .builder
                            .build_cast(
                                inkwell::values::InstructionOpcode::BitCast,
                                lhs,
                                this.f64_ty,
                                "cast\0",
                            )
                            .into_float_value();
                        let r = this.f64_ty.const_float(right);

                        let f = this.builder.build_float_mul(l, r, "mul\0");
                        this.builder
                            .build_cast(
                                inkwell::values::InstructionOpcode::BitCast,
                                f,
                                this.i64_ty,
                                "cast\0",
                            )
                            .into_int_value()
                    },
                );
            }
            OpCode::MulImmF32 { result, left, right } => {
                self.binary_imm(
                    left,
                    right as f64,
                    result,
                    operations::mul,
                    "mul",
                    |this, lhs, right| {
                        let l = this
                            .builder
                            .build_cast(
                                inkwell::values::InstructionOpcode::BitCast,
                                lhs,
                                this.f64_ty,
                                "cast\0",
                            )
                            .into_float_value();
                        let r = this.f64_ty.const_float(right);

                        let f = this.builder.build_float_mul(l, r, "mul\0");
                        this.builder
                            .build_cast(
                                inkwell::values::InstructionOpcode::BitCast,
                                f,
                                this.i64_ty,
                                "cast\0",
                            )
                            .into_int_value()
                    },
                );
            }
            OpCode::Div {
                result,
                left,
                right,
            } => {
                self.binary(
                    left,
                    right,
                    result,
                    operations::div,
                    "div",
                    |this, lhs, rhs| {
                        let l = this
                            .builder
                            .build_cast(
                                inkwell::values::InstructionOpcode::BitCast,
                                lhs,
                                this.f64_ty,
                                "cast\0",
                            )
                            .into_float_value();
                        let r = this
                            .builder
                            .build_cast(
                                inkwell::values::InstructionOpcode::BitCast,
                                rhs,
                                this.f64_ty,
                                "cast\0",
                            )
                            .into_float_value();

                        let f = this.builder.build_float_div(l, r, "add\0");
                        this.builder
                            .build_cast(
                                inkwell::values::InstructionOpcode::BitCast,
                                f,
                                this.i64_ty,
                                "cast\0",
                            )
                            .into_int_value()
                    },
                );
            }
            OpCode::DivImmI32 { result, left, right } => {
                self.binary_imm(
                    left,
                    right as f64,
                    result,
                    operations::div,
                    "div",
                    |this, lhs, right| {
                        let l = this
                            .builder
                            .build_cast(
                                inkwell::values::InstructionOpcode::BitCast,
                                lhs,
                                this.f64_ty,
                                "cast\0",
                            )
                            .into_float_value();
                        let r = this.f64_ty.const_float(right);

                        let f = this.builder.build_float_div(l, r, "div\0");
                        this.builder
                            .build_cast(
                                inkwell::values::InstructionOpcode::BitCast,
                                f,
                                this.i64_ty,
                                "cast\0",
                            )
                            .into_int_value()
                    },
                );
            }
            OpCode::DivImmF32 { result, left, right } => {
                self.binary_imm(
                    left,
                    right as f64,
                    result,
                    operations::div,
                    "div",
                    |this, lhs, right| {
                        let l = this
                            .builder
                            .build_cast(
                                inkwell::values::InstructionOpcode::BitCast,
                                lhs,
                                this.f64_ty,
                                "cast\0",
                            )
                            .into_float_value();
                        let r = this.f64_ty.const_float(right);

                        let f = this.builder.build_float_div(l, r, "div\0");
                        this.builder
                            .build_cast(
                                inkwell::values::InstructionOpcode::BitCast,
                                f,
                                this.i64_ty,
                                "cast\0",
                            )
                            .into_int_value()
                    },
                );
            }
            OpCode::Rem {
                result,
                left,
                right,
            } => {
                self.binary(
                    left,
                    right,
                    result,
                    operations::rem,
                    "rem",
                    |this, lhs, rhs| {
                        let l = this
                            .builder
                            .build_cast(
                                inkwell::values::InstructionOpcode::BitCast,
                                lhs,
                                this.f64_ty,
                                "cast\0",
                            )
                            .into_float_value();
                        let r = this
                            .builder
                            .build_cast(
                                inkwell::values::InstructionOpcode::BitCast,
                                rhs,
                                this.f64_ty,
                                "cast\0",
                            )
                            .into_float_value();

                        let f = this.builder.build_float_rem(l, r, "add\0");
                        this.builder
                            .build_cast(
                                inkwell::values::InstructionOpcode::BitCast,
                                f,
                                this.i64_ty,
                                "cast\0",
                            )
                            .into_int_value()
                    },
                );
            }
            OpCode::RemImmI32 { result, left, right } => {
                self.binary_imm(
                    left,
                    right as f64,
                    result,
                    operations::rem,
                    "rem",
                    |this, lhs, right| {
                        let l = this
                            .builder
                            .build_cast(
                                inkwell::values::InstructionOpcode::BitCast,
                                lhs,
                                this.f64_ty,
                                "cast\0",
                            )
                            .into_float_value();
                        let r = this.f64_ty.const_float(right);

                        let f = this.builder.build_float_rem(l, r, "rem\0");
                        this.builder
                            .build_cast(
                                inkwell::values::InstructionOpcode::BitCast,
                                f,
                                this.i64_ty,
                                "cast\0",
                            )
                            .into_int_value()
                    },
                );
            }
            OpCode::RemImmF32 { result, left, right } => {
                self.binary_imm(
                    left,
                    right as f64,
                    result,
                    operations::rem,
                    "rem",
                    |this, lhs, right| {
                        let l = this
                            .builder
                            .build_cast(
                                inkwell::values::InstructionOpcode::BitCast,
                                lhs,
                                this.f64_ty,
                                "cast\0",
                            )
                            .into_float_value();
                        let r = this.f64_ty.const_float(right);

                        let f = this.builder.build_float_rem(l, r, "rem\0");
                        this.builder
                            .build_cast(
                                inkwell::values::InstructionOpcode::BitCast,
                                f,
                                this.i64_ty,
                                "cast\0",
                            )
                            .into_int_value()
                    },
                );
            }
            OpCode::Lt {
                result,
                left,
                right,
            } => {
                self.binary(
                    left,
                    right,
                    result,
                    operations::lt,
                    "lt",
                    |this, lhs, rhs| {
                        let l = this
                            .builder
                            .build_cast(
                                inkwell::values::InstructionOpcode::BitCast,
                                lhs,
                                this.f64_ty,
                                "cast\0",
                            )
                            .into_float_value();
                        let r = this
                            .builder
                            .build_cast(
                                inkwell::values::InstructionOpcode::BitCast,
                                rhs,
                                this.f64_ty,
                                "cast\0",
                            )
                            .into_float_value();

                        let is_lt = this.builder.build_float_compare(
                            inkwell::FloatPredicate::OLT,
                            l,
                            r,
                            "lt\0",
                        );
                        let t = this.i64_ty.const_int(JValue::TRUE_TAG, false);
                        let f = this.i64_ty.const_int(JValue::FALSE_TAG, false);

                        this.builder
                            .build_select(is_lt, t, f, "is_lt\0")
                            .into_int_value()
                    },
                );
            }
            OpCode::LtImmI32 { result, left, right } => {
                self.binary_imm(
                    left,
                    right as f64,
                    result,
                    operations::lt,
                    "lt",
                    |this, lhs, right| {
                        let l = this
                            .builder
                            .build_cast(
                                inkwell::values::InstructionOpcode::BitCast,
                                lhs,
                                this.f64_ty,
                                "cast\0",
                            )
                            .into_float_value();
                        let r = this.f64_ty.const_float(right);

                        let is_lt = this.builder.build_float_compare(inkwell::FloatPredicate::OLT,l, r, "lt\0");
                        
                        let t = this.i64_ty.const_int(JValue::TRUE_TAG, false);
                        let f = this.i64_ty.const_int(JValue::FALSE_TAG, false);

                        this.builder
                            .build_select(is_lt, t, f, "is_lt\0")
                            .into_int_value()
                    },
                );
            }
            OpCode::LtImmF32 { result, left, right } => {
                self.binary_imm(
                    left,
                    right as f64,
                    result,
                    operations::lt,
                    "lt",
                    |this, lhs, right| {
                        let l = this
                            .builder
                            .build_cast(
                                inkwell::values::InstructionOpcode::BitCast,
                                lhs,
                                this.f64_ty,
                                "cast\0",
                            )
                            .into_float_value();
                        let r = this.f64_ty.const_float(right);

                        let is_lt = this.builder.build_float_compare(inkwell::FloatPredicate::OLT,l, r, "lt\0");
                        
                        let t = this.i64_ty.const_int(JValue::TRUE_TAG, false);
                        let f = this.i64_ty.const_int(JValue::FALSE_TAG, false);

                        this.builder
                            .build_select(is_lt, t, f, "is_lt\0")
                            .into_int_value()
                    },
                );
            }
            OpCode::LtEq {
                result,
                left,
                right,
            } => {
                self.binary(
                    left,
                    right,
                    result,
                    operations::lteq,
                    "lteq",
                    |this, lhs, rhs| {
                        let l = this
                            .builder
                            .build_cast(
                                inkwell::values::InstructionOpcode::BitCast,
                                lhs,
                                this.f64_ty,
                                "cast\0",
                            )
                            .into_float_value();
                        let r = this
                            .builder
                            .build_cast(
                                inkwell::values::InstructionOpcode::BitCast,
                                rhs,
                                this.f64_ty,
                                "cast\0",
                            )
                            .into_float_value();

                        let is_lt = this.builder.build_float_compare(
                            inkwell::FloatPredicate::OLE,
                            l,
                            r,
                            "lt\0",
                        );
                        let t = this.i64_ty.const_int(JValue::TRUE_TAG, false);
                        let f = this.i64_ty.const_int(JValue::FALSE_TAG, false);

                        this.builder
                            .build_select(is_lt, t, f, "is_lt\0")
                            .into_int_value()
                    },
                );
            }
            OpCode::LtEqImmI32 { result, left, right } => {
                self.binary_imm(
                    left,
                    right as f64,
                    result,
                    operations::lteq,
                    "lteq",
                    |this, lhs, right| {
                        let l = this
                            .builder
                            .build_cast(
                                inkwell::values::InstructionOpcode::BitCast,
                                lhs,
                                this.f64_ty,
                                "cast\0",
                            )
                            .into_float_value();
                        let r = this.f64_ty.const_float(right);

                        let is_lt = this.builder.build_float_compare(inkwell::FloatPredicate::OLE,l, r, "lteq\0");
                        
                        let t = this.i64_ty.const_int(JValue::TRUE_TAG, false);
                        let f = this.i64_ty.const_int(JValue::FALSE_TAG, false);

                        this.builder
                            .build_select(is_lt, t, f, "is_lt\0")
                            .into_int_value()
                    },
                );
            }
            OpCode::LtEqImmF32 { result, left, right } => {
                self.binary_imm(
                    left,
                    right as f64,
                    result,
                    operations::lteq,
                    "lteq",
                    |this, lhs, right| {
                        let l = this
                            .builder
                            .build_cast(
                                inkwell::values::InstructionOpcode::BitCast,
                                lhs,
                                this.f64_ty,
                                "cast\0",
                            )
                            .into_float_value();
                        let r = this.f64_ty.const_float(right);

                        let is_lt = this.builder.build_float_compare(inkwell::FloatPredicate::OLE,l, r, "lteq\0");
                        
                        let t = this.i64_ty.const_int(JValue::TRUE_TAG, false);
                        let f = this.i64_ty.const_int(JValue::FALSE_TAG, false);

                        this.builder
                            .build_select(is_lt, t, f, "is_lt\0")
                            .into_int_value()
                    },
                );
            }
            OpCode::Gt {
                result,
                left,
                right,
            } => {
                self.binary(
                    left,
                    right,
                    result,
                    operations::gt,
                    "gt",
                    |this, lhs, rhs| {
                        let l = this
                            .builder
                            .build_cast(
                                inkwell::values::InstructionOpcode::BitCast,
                                lhs,
                                this.f64_ty,
                                "cast\0",
                            )
                            .into_float_value();
                        let r = this
                            .builder
                            .build_cast(
                                inkwell::values::InstructionOpcode::BitCast,
                                rhs,
                                this.f64_ty,
                                "cast\0",
                            )
                            .into_float_value();

                        let is_lt = this.builder.build_float_compare(
                            inkwell::FloatPredicate::OGT,
                            l,
                            r,
                            "lt\0",
                        );
                        let t = this.i64_ty.const_int(JValue::TRUE_TAG, false);
                        let f = this.i64_ty.const_int(JValue::FALSE_TAG, false);

                        this.builder
                            .build_select(is_lt, t, f, "is_lt\0")
                            .into_int_value()
                    },
                );
            }
            OpCode::GtImmI32 { result, left, right } => {
                self.binary_imm(
                    left,
                    right as f64,
                    result,
                    operations::gt,
                    "gt",
                    |this, lhs, right| {
                        let l = this
                            .builder
                            .build_cast(
                                inkwell::values::InstructionOpcode::BitCast,
                                lhs,
                                this.f64_ty,
                                "cast\0",
                            )
                            .into_float_value();
                        let r = this.f64_ty.const_float(right);

                        let is_gt = this.builder.build_float_compare(inkwell::FloatPredicate::OGT,l, r, "gt\0");
                        
                        let t = this.i64_ty.const_int(JValue::TRUE_TAG, false);
                        let f = this.i64_ty.const_int(JValue::FALSE_TAG, false);

                        this.builder
                            .build_select(is_gt, t, f, "is_lt\0")
                            .into_int_value()
                    },
                );
            }
            OpCode::GtImmF32 { result, left, right } => {
                self.binary_imm(
                    left,
                    right as f64,
                    result,
                    operations::gt,
                    "gt",
                    |this, lhs, right| {
                        let l = this
                            .builder
                            .build_cast(
                                inkwell::values::InstructionOpcode::BitCast,
                                lhs,
                                this.f64_ty,
                                "cast\0",
                            )
                            .into_float_value();
                        let r = this.f64_ty.const_float(right);

                        let is_gt = this.builder.build_float_compare(inkwell::FloatPredicate::OGT,l, r, "gt\0");
                        
                        let t = this.i64_ty.const_int(JValue::TRUE_TAG, false);
                        let f = this.i64_ty.const_int(JValue::FALSE_TAG, false);

                        this.builder
                            .build_select(is_gt, t, f, "is_lt\0")
                            .into_int_value()
                    },
                );
            }
            OpCode::GtEq {
                result,
                left,
                right,
            } => {
                self.binary(
                    left,
                    right,
                    result,
                    operations::gteq,
                    "gteq",
                    |this, lhs, rhs| {
                        let l = this
                            .builder
                            .build_cast(
                                inkwell::values::InstructionOpcode::BitCast,
                                lhs,
                                this.f64_ty,
                                "cast\0",
                            )
                            .into_float_value();
                        let r = this
                            .builder
                            .build_cast(
                                inkwell::values::InstructionOpcode::BitCast,
                                rhs,
                                this.f64_ty,
                                "cast\0",
                            )
                            .into_float_value();

                        let is_gt = this.builder.build_float_compare(
                            inkwell::FloatPredicate::OGE,
                            l,
                            r,
                            "lt\0",
                        );
                        let t = this.i64_ty.const_int(JValue::TRUE_TAG, false);
                        let f = this.i64_ty.const_int(JValue::FALSE_TAG, false);

                        this.builder
                            .build_select(is_gt, t, f, "is_lt\0")
                            .into_int_value()
                    },
                );
            }
            OpCode::GtEqImmI32 { result, left, right } => {
                self.binary_imm(
                    left,
                    right as f64,
                    result,
                    operations::gteq,
                    "gteq",
                    |this, lhs, right| {
                        let l = this
                            .builder
                            .build_cast(
                                inkwell::values::InstructionOpcode::BitCast,
                                lhs,
                                this.f64_ty,
                                "cast\0",
                            )
                            .into_float_value();
                        let r = this.f64_ty.const_float(right);

                        let is_gt = this.builder.build_float_compare(inkwell::FloatPredicate::OGE,l, r, "gteq\0");
                        
                        let t = this.i64_ty.const_int(JValue::TRUE_TAG, false);
                        let f = this.i64_ty.const_int(JValue::FALSE_TAG, false);

                        this.builder
                            .build_select(is_gt, t, f, "is_lt\0")
                            .into_int_value()
                    },
                );
            }
            OpCode::GtEqImmF32 { result, left, right } => {
                self.binary_imm(
                    left,
                    right as f64,
                    result,
                    operations::gteq,
                    "gteq",
                    |this, lhs, right| {
                        let l = this
                            .builder
                            .build_cast(
                                inkwell::values::InstructionOpcode::BitCast,
                                lhs,
                                this.f64_ty,
                                "cast\0",
                            )
                            .into_float_value();
                        let r = this.f64_ty.const_float(right);

                        let is_gt = this.builder.build_float_compare(inkwell::FloatPredicate::OGE,l, r, "gteq\0");
                        
                        let t = this.i64_ty.const_int(JValue::TRUE_TAG, false);
                        let f = this.i64_ty.const_int(JValue::FALSE_TAG, false);

                        this.builder
                            .build_select(is_gt, t, f, "is_lt\0")
                            .into_int_value()
                    },
                );
            }
            OpCode::EqEqEq { result, left, right } => {
                let lhs = self.read_reg(left);
                let rhs = self.read_reg(right);

                let is_eq = self.builder.build_int_compare(inkwell::IntPredicate::EQ, lhs, rhs, "eqeqeq\0");

                let t = self.i64_ty.const_int(JValue::TRUE_TAG, false);
                let f = self.i64_ty.const_int(JValue::FALSE_TAG, false);

                let value = self.builder
                    .build_select(is_eq, t, f, "is_lt\0")
                    .into_int_value();

                self.store_reg(result, value);
            }
            OpCode::EqEqEqImmI32 { result, left, right } => {
                let lhs = self.read_reg(left);
                let rhs = self.i64_ty.const_int(JValue::number(right as f64).to_bits(), false);

                let is_eq = self.builder.build_int_compare(inkwell::IntPredicate::EQ, lhs, rhs, "eqeqeq\0");

                let t = self.i64_ty.const_int(JValue::TRUE_TAG, false);
                let f = self.i64_ty.const_int(JValue::FALSE_TAG, false);

                let value = self.builder
                    .build_select(is_eq, t, f, "is_lt\0")
                    .into_int_value();

                self.store_reg(result, value);
            }
            OpCode::EqEqEqImmF32 { result, left, right } => {
                let lhs = self.read_reg(left);
                let rhs = self.i64_ty.const_int(JValue::number(right as f64).to_bits(), false);

                let is_eq = self.builder.build_int_compare(inkwell::IntPredicate::EQ, lhs, rhs, "eqeqeq\0");

                let t = self.i64_ty.const_int(JValue::TRUE_TAG, false);
                let f = self.i64_ty.const_int(JValue::FALSE_TAG, false);

                let value = self.builder
                    .build_select(is_eq, t, f, "is_lt\0")
                    .into_int_value();

                self.store_reg(result, value);
            }
            OpCode::EqEq {
                result,
                left,
                right,
            } => {
                self.binary(left, right, result, operations::eqeq, "eqeq", 
                |this, lhs, rhs|{
                    let is_eq = this.builder.build_int_compare(inkwell::IntPredicate::EQ, lhs, rhs, "compare\0");
                    let t = this.i64_ty.const_int(JValue::TRUE_TAG, false);
                    let f = this.i64_ty.const_int(JValue::FALSE_TAG, false);
                    this.builder.build_select(is_eq, t, f, "eqeq\0").into_int_value()
                });
            }
            OpCode::EqEqImmI32 {
                result,
                left,
                right,
            } => {
                self.binary_imm(
                    left,
                    right as f64,
                    result,
                    operations::eqeq,
                    "eqeq",
                    |this, lhs, right| {
                        let l = this
                            .builder
                            .build_cast(
                                inkwell::values::InstructionOpcode::BitCast,
                                lhs,
                                this.f64_ty,
                                "cast\0",
                            )
                            .into_float_value();
                        let r = this.f64_ty.const_float(right);

                        let b = this.builder.build_float_compare(
                            inkwell::FloatPredicate::OEQ,
                            l,
                            r,
                            "feqeq\0",
                        );

                        let t = this.i64_ty.const_int(JValue::TRUE_TAG, false);
                        let f = this.i64_ty.const_int(JValue::FALSE_TAG, false);

                        this.builder
                            .build_select(b, t, f, "feqeq\0")
                            .into_int_value()
                    },
                );
            }
            OpCode::EqEqImmF32 {
                result,
                left,
                right,
            } => {
                self.binary_imm(
                    left,
                    right as f64,
                    result,
                    operations::eqeq,
                    "eqeq",
                    |this, lhs, right| {
                        let l = this
                            .builder
                            .build_cast(
                                inkwell::values::InstructionOpcode::BitCast,
                                lhs,
                                this.f64_ty,
                                "cast\0",
                            )
                            .into_float_value();
                        let r = this.f64_ty.const_float(right);

                        let b = this.builder.build_float_compare(
                            inkwell::FloatPredicate::OEQ,
                            l,
                            r,
                            "feqeq\0",
                        );

                        let t = this.i64_ty.const_int(JValue::TRUE_TAG, false);
                        let f = this.i64_ty.const_int(JValue::FALSE_TAG, false);

                        this.builder
                            .build_select(b, t, f, "feqeq\0")
                            .into_int_value()
                    },
                );
            }
            e => {}
        };
    }
}
