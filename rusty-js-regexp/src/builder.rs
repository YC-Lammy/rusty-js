
use cranelift::codegen::isa;
use cranelift::codegen::ir;
use cranelift::frontend::{
    FunctionBuilder,
    FunctionBuilderContext,
    Switch,
    Variable
};
use cranelift::prelude::*;

use crate::ast::*;

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

#[cfg(target_pointer_width="64")]
const USIZE:types::Type = types::I64;

#[cfg(target_pointer_width="32")]
const USIZE:types::Type = types::I32;

#[derive(Clone, Copy)]
pub struct RegExpRecord {
    pub ignore_case: bool,
    pub multiline: bool,
    pub dot_all: bool,
    pub unicode: bool,
    pub capturing_groups_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Forward,
    Backward,
}

impl Direction {
    pub fn is_forward(self) -> bool {
        self == Self::Forward
    }

    pub fn is_backward(self) -> bool {
        self == Self::Backward
    }
}

pub struct Builder{
    builder:FunctionBuilder<'static>,

    top_level:Option<Disjunction>,

    error:Variable,
    start_index:Variable,
    end_index:Variable,
    /// *mut \[usize;2]
    captures:Variable,

    /// *mut char
    input:Variable,
    /// usize
    last_index:Variable,
}

impl Builder{
    pub fn new() -> Self{
        let func = Box::leak(Box::new(ir::Function::new()));
        let ctx = Box::leak(Box::new(FunctionBuilderContext::new()));

        let builder = FunctionBuilder::new(func, ctx);

        let mut this = Self{
            builder:builder,

            top_level:None,

            error:Variable::with_u32(0),
            start_index:Variable::with_u32(1),
            end_index:Variable::with_u32(2),
            captures:Variable::with_u32(3),

            input:Variable::with_u32(4),
            last_index:Variable::with_u32(5),
        };

        let start = this.builder.create_block();
        this.builder.append_block_params_for_function_params(start);
        this.builder.switch_to_block(start);

        return this
    }

    pub fn create_matcher(&mut self, disjunction:Disjunction, recod:RegExpRecord){

        self.top_level = Some(disjunction);
        let disjunction = unsafe{std::mem::transmute_copy(&self.top_level.as_ref().unwrap())};

        self.translate_disjunction(disjunction, recod);
    }

    pub fn translate_disjunction(&mut self, disjunction:&Disjunction, recod:RegExpRecord){
        let exit = self.builder.create_block();

        for a in &disjunction.0{
            self.translate_alternative(a, recod, Direction::Forward);
            let error = self.builder.use_var(self.error);
            // jump to exit if suceeded
            self.builder.ins().brnz(error, exit, &[]);
        }

        self.builder.ins().jump(exit, &[]);
        self.builder.seal_block(exit);

        self.builder.switch_to_block(exit);
    }

    pub fn translate_alternative(&mut self, alt:&Alternative, recod:RegExpRecord, direction:Direction){
        
    }
}