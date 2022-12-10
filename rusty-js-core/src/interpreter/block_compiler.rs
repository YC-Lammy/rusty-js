use std::collections::HashMap;

use cranelift::codegen::ir;
use cranelift::prelude::isa;
use cranelift::prelude::types;
use cranelift::prelude::AbiParam;
use cranelift::prelude::FunctionBuilder;
use cranelift::prelude::FunctionBuilderContext;
use cranelift::prelude::InstBuilder;
use cranelift::prelude::StackSlotData;
use cranelift::prelude::Variable;

use crate::bytecodes::Block;
use crate::bytecodes::OpCode;
use crate::JValue;

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

#[cfg(target_pointer_width = "16")]
const POINTER_TYPE: types::Type = types::I16;

#[cfg(target_pointer_width = "32")]
const POINTER_TYPE: types::Type = types::I32;

#[cfg(target_pointer_width = "64")]
const POINTER_TYPE: types::Type = types::I64;

#[cfg(target_pointer_width = "128")]
const POINTER_TYPE: types::Type = types::I128;

pub struct CompiledBlock {
    /// return (reg0, reg1, reg2, value, is_error, is_return, need_jump)
    pub func: fn(
        reg0: JValue,
        reg1: JValue,
        reg2: JValue,
        this: *mut JValue,
        args: *const JValue,
        argc: usize,
        stack: *mut JValue,
    ) -> (JValue, JValue, JValue, JValue, bool, bool, u32),
    mem: memmap2::Mmap,
}

pub fn compile_block(start: usize, opcodes: &[OpCode]) -> CompiledBlock {
    // blocks that are created within this block
    let mut compiler = BlockCompiler::new();
    return compiler.compile(start, opcodes);
}

struct BlockCompiler {
    created_block: HashMap<Block, ir::Block>,
    registers: [(Variable, Variable); 3],
    this_ptr: Variable,
    args_ptr: Variable,
    argc: Variable,
    stack_ptr: Variable,

    temp: Vec<ir::StackSlot>,
}

impl BlockCompiler {
    pub fn new() -> Self {
        Self {
            created_block: Default::default(),
            registers: [
                (Variable::with_u32(0), Variable::with_u32(1)),
                (Variable::with_u32(2), Variable::with_u32(3)),
                (Variable::with_u32(4), Variable::with_u32(5)),
            ],
            temp: Default::default(),
            this_ptr: Variable::with_u32(6),
            args_ptr: Variable::with_u32(7),
            argc: Variable::with_u32(8),
            stack_ptr: Variable::with_u32(9),
        }
    }

    fn compile(&mut self, start: usize, opcodes: &[OpCode]) -> CompiledBlock {
        let mut func_ctx = FunctionBuilderContext::new();
        let mut func = ir::Function::new();
        let mut builder = FunctionBuilder::new(&mut func, &mut func_ctx);

        let pointer_type = ISA.pointer_type();

        let mut i = start;

        builder.declare_var(Variable::with_u32(0), POINTER_TYPE);
        builder.declare_var(Variable::with_u32(1), POINTER_TYPE);
        builder.declare_var(Variable::with_u32(2), POINTER_TYPE);
        builder.declare_var(Variable::with_u32(3), POINTER_TYPE);
        builder.declare_var(Variable::with_u32(4), POINTER_TYPE);
        builder.declare_var(Variable::with_u32(5), POINTER_TYPE);
        builder.declare_var(Variable::with_u32(6), POINTER_TYPE);
        builder.declare_var(Variable::with_u32(7), POINTER_TYPE);
        builder.declare_var(Variable::with_u32(8), POINTER_TYPE);
        builder.declare_var(Variable::with_u32(9), POINTER_TYPE);

        builder.func.signature.call_conv = ISA.default_call_conv();
        builder.func.signature.params.extend_from_slice(&[
            AbiParam::new(pointer_type),
            AbiParam::new(pointer_type),
            AbiParam::new(pointer_type),
            AbiParam::new(pointer_type),
            AbiParam::new(pointer_type),
            AbiParam::new(pointer_type),
            AbiParam::new(pointer_type),
            AbiParam::new(pointer_type),
            AbiParam::new(pointer_type),
        ]);
        builder.func.signature.returns.extend_from_slice(&[
            AbiParam::new(pointer_type),
            AbiParam::new(pointer_type),
            AbiParam::new(pointer_type),
            AbiParam::new(pointer_type),
            AbiParam::new(pointer_type),
            AbiParam::new(pointer_type),
            AbiParam::new(pointer_type),
            AbiParam::new(pointer_type),
            AbiParam::new(types::B8),
            AbiParam::new(types::B8),
            AbiParam::new(types::I32),
        ]);

        let start = builder.create_block();
        builder.append_block_params_for_function_params(start);
        builder.switch_to_block(start);

        let params = builder.block_params(start).to_vec();

        builder.def_var(Variable::with_u32(0), params[0]);
        builder.def_var(Variable::with_u32(1), params[1]);
        builder.def_var(Variable::with_u32(2), params[2]);
        builder.def_var(Variable::with_u32(3), params[3]);
        builder.def_var(Variable::with_u32(4), params[4]);
        builder.def_var(Variable::with_u32(5), params[5]);
        builder.def_var(Variable::with_u32(6), params[6]);
        builder.def_var(Variable::with_u32(7), params[7]);
        builder.def_var(Variable::with_u32(8), params[8]);
        builder.def_var(Variable::with_u32(9), params[9]);

        let exit_block = builder.create_block();
        builder.append_block_param(exit_block, types::B8);
        builder.append_block_param(exit_block, types::B8);
        builder.append_block_param(exit_block, ISA.pointer_type());
        builder.append_block_param(exit_block, ISA.pointer_type());
        builder.append_block_param(exit_block, types::I32);

        loop {
            if i == opcodes.len() {
                break;
            }

            let code = opcodes[i];

            match code {
                OpCode::Debugger => {}
                OpCode::Return { value } => {
                    let is_err = builder.ins().bconst(types::B8, false);
                    let is_return = builder.ins().bconst(types::B8, true);
                    let vt = builder.use_var(self.registers[value.0 as usize].0);
                    let v = builder.use_var(self.registers[value.0 as usize].1);
                    let need_jump = builder.ins().iconst(types::I32, 0);

                    builder
                        .ins()
                        .jump(exit_block, &[is_err, is_return, vt, v, need_jump]);
                }
                OpCode::Jump { to, line } => {
                    if let Some(block) = self.created_block.get(&to) {
                        builder.ins().jump(*block, &[]);
                    } else {
                        let is_err = builder.ins().bconst(types::B8, false);
                        let is_return = builder.ins().bconst(types::B8, false);
                        let vt = builder.ins().iconst(POINTER_TYPE, 0);
                        let v = builder.ins().iconst(POINTER_TYPE, 0);
                        let need_jump = builder.ins().iconst(types::I32, to.0 as i64);

                        builder
                            .ins()
                            .jump(exit_block, &[is_err, is_return, vt, v, need_jump]);
                    }
                }
                OpCode::JumpIfFalse { value, to, line } => {
                    let vt = builder.use_var(self.registers[value.0 as usize].0);
                    let v = builder.use_var(self.registers[value.0 as usize].1);

                    if let Some(block) = self.created_block.get(&to) {
                        builder.ins().brz(v, *block, &[]);
                    } else {
                        let is_err = builder.ins().bconst(types::B8, false);
                        let is_return = builder.ins().bconst(types::B8, false);
                        let vt = builder.ins().iconst(POINTER_TYPE, 0);
                        let v = builder.ins().iconst(POINTER_TYPE, 0);
                        let need_jump = builder.ins().iconst(types::I32, to.0 as i64);

                        builder
                            .ins()
                            .brz(v, exit_block, &[is_err, is_return, vt, v, need_jump]);
                    }
                }
                OpCode::JumpIfTrue { value, to, line } => {
                    let vt = builder.use_var(self.registers[value.0 as usize].0);
                    let v = builder.use_var(self.registers[value.0 as usize].1);

                    if let Some(block) = self.created_block.get(&to) {
                        builder.ins().brnz(v, *block, &[]);
                    } else {
                        let is_err = builder.ins().bconst(types::B8, false);
                        let is_return = builder.ins().bconst(types::B8, false);
                        let vt = builder.ins().iconst(POINTER_TYPE, 0);
                        let v = builder.ins().iconst(POINTER_TYPE, 0);
                        let need_jump = builder.ins().iconst(types::I32, to.0 as i64);

                        builder
                            .ins()
                            .brnz(v, exit_block, &[is_err, is_return, vt, v, need_jump]);
                    }
                }
                OpCode::CreateBlock(b) => {
                    let a = builder.create_block();
                    self.created_block.insert(b, a);
                }
                OpCode::SwitchToBlock(b) => {
                    if let Some(block) = self.created_block.get(&b) {
                        builder.switch_to_block(*block);
                    } else {
                        break;
                    };
                }

                // var
                OpCode::DeclareDynamicVar { from, kind, offset } => {}
                OpCode::ReadDynamicVar { result, id } => {}
                OpCode::WriteDynamicVar { from, id } => {}

                // memory
                OpCode::StoreTemp { value } => {
                    let vt = builder.use_var(self.registers[value.0 as usize].0);
                    let v = builder.use_var(self.registers[value.0 as usize].1);

                    let slot = builder.create_sized_stack_slot(StackSlotData {
                        kind: cranelift::prelude::StackSlotKind::ExplicitSlot,
                        size: JValue::SIZE as u32,
                    });

                    self.temp.push(slot);

                    builder.ins().stack_store(vt, slot, 0);
                }
                OpCode::ReadTemp { value } => {
                    let slot = self.temp.last().unwrap();

                    let v = builder.ins().stack_load(POINTER_TYPE, *slot, 0);

                    builder.def_var(self.registers[value.0 as usize].1, v);
                }
                OpCode::ReleaseTemp => {
                    self.temp.pop().unwrap();
                }
                _ => {}
            }
            i += 1;
        }

        builder.seal_all_blocks();

        let re = ISA.compile_function(&func, false).unwrap();

        let data = re.buffer.data();
        let mut mem = memmap2::MmapMut::map_anon(data.len()).unwrap();
        unsafe { std::ptr::copy_nonoverlapping(data.as_ptr(), mem.as_mut_ptr(), data.len()) };

        let mem = mem.make_exec().unwrap();
        let f = unsafe { std::mem::transmute(mem.as_ptr()) };

        CompiledBlock { func: f, mem: mem }
    }
}
