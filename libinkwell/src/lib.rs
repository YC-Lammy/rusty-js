//! Bindings to LLVM's C API.
//!
//! Refer to the [LLVM documentation](http://llvm.org/docs/) for more
//! information.

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(warnings)]
#![no_std]

extern crate libc;

use self::prelude::*;

#[derive(Debug)]
pub enum LLVMMemoryBuffer {}

#[derive(Debug)]
pub enum LLVMContext {}

#[derive(Debug)]
pub enum LLVMModule {}

#[derive(Debug)]
pub enum LLVMType {}

#[derive(Debug)]
pub enum LLVMValue {}

#[derive(Debug)]
pub enum LLVMBasicBlock {}

#[derive(Debug)]
pub enum LLVMOpaqueMetadata {}

#[derive(Debug)]
pub enum LLVMOpaqueNamedMDNode {}

#[derive(Debug)]
pub enum LLVMOpaqueValueMetadataEntry {}

#[derive(Debug)]
pub enum LLVMBuilder {}

#[derive(Debug)]
pub enum LLVMOpaqueDIBuilder {}

#[derive(Debug)]
pub enum LLVMModuleProvider {}

#[derive(Debug)]
pub enum LLVMPassManager {}

#[derive(Debug)]
pub enum LLVMPassRegistry {}

#[derive(Debug)]
pub enum LLVMUse {}

#[derive(Debug)]
pub enum LLVMDiagnosticInfo {}

#[derive(Debug)]
pub enum LLVMComdat {}

#[derive(Debug)]
pub enum LLVMOpaqueModuleFlagEntry {}

#[derive(Debug)]
pub enum LLVMOpaqueJITEventListener {}

#[derive(Debug)]
pub enum LLVMOpaqueAttributeRef {}

/// Core types used throughout LLVM.
///
/// In most cases you will want to `use llvm::prelude::*`.
pub mod prelude {
    pub type LLVMBool = ::libc::c_int;
    pub type LLVMMemoryBufferRef = *mut super::LLVMMemoryBuffer;
    pub type LLVMContextRef = *mut super::LLVMContext;
    pub type LLVMModuleRef = *mut super::LLVMModule;
    pub type LLVMTypeRef = *mut super::LLVMType;
    pub type LLVMValueRef = *mut super::LLVMValue;
    pub type LLVMBasicBlockRef = *mut super::LLVMBasicBlock;
    pub type LLVMMetadataRef = *mut super::LLVMOpaqueMetadata;
    pub type LLVMNamedMDNodeRef = *mut super::LLVMOpaqueNamedMDNode;
    pub type LLVMValueMetadataEntry = *mut super::LLVMOpaqueValueMetadataEntry;
    pub type LLVMBuilderRef = *mut super::LLVMBuilder;
    pub type LLVMDIBuilderRef = *mut super::LLVMOpaqueDIBuilder;
    pub type LLVMModuleProviderRef = *mut super::LLVMModuleProvider;
    pub type LLVMPassManagerRef = *mut super::LLVMPassManager;
    pub type LLVMPassRegistryRef = *mut super::LLVMPassRegistry;
    pub type LLVMUseRef = *mut super::LLVMUse;
    pub type LLVMDiagnosticInfoRef = *mut super::LLVMDiagnosticInfo;
    pub type LLVMComdatRef = *mut super::LLVMComdat;
    pub type LLVMModuleFlagEntry = *mut super::LLVMOpaqueModuleFlagEntry;
    pub type LLVMJITEventListenerRef = *mut super::LLVMOpaqueJITEventListener;
    pub type LLVMAttributeRef = *mut super::LLVMOpaqueAttributeRef;
}

pub mod analysis;
pub mod bit_reader;
pub mod bit_writer;
pub mod comdat;
pub mod core;
pub mod debuginfo;
pub mod disassembler;
pub mod error;
pub mod error_handling;
pub mod execution_engine;
pub mod initialization;
pub mod ir_reader;
pub mod linker;
pub mod lto;
pub mod object;
pub mod orc2;
pub mod remarks;
pub mod support;
pub mod target;
pub mod target_machine;

pub mod transforms {
    pub mod aggressive_instcombine;
    pub mod coroutines;
    pub mod instcombine;
    pub mod ipo;
    pub mod pass_builder;
    pub mod pass_manager_builder;
    pub mod scalar;
    pub mod util;
    pub mod vectorize;
}

pub use llvm_sys::{
    LLVMAtomicOrdering, LLVMAtomicRMWBinOp, LLVMAttributeFunctionIndex, LLVMAttributeIndex,
    LLVMAttributeReturnIndex, LLVMCallConv, LLVMDLLStorageClass, LLVMDiagnosticHandler,
    LLVMDiagnosticSeverity, LLVMInlineAsmDialect, LLVMIntPredicate, LLVMLandingPadClauseTy,
    LLVMLinkage, LLVMModuleFlagBehavior, LLVMOpcode, LLVMRealPredicate, LLVMThreadLocalMode,
    LLVMTypeKind, LLVMUnnamedAddr, LLVMValueKind, LLVMVisibility, LLVMYieldCallback,
};
