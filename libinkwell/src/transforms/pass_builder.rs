#![allow(non_snake_case)]

use llvm_sys::error::LLVMErrorRef;
use llvm_sys::prelude::*;
use llvm_sys::target_machine::LLVMTargetMachineRef;

pub use llvm_sys::transforms::pass_builder::LLVMOpaquePassBuilderOptions;
pub use llvm_sys::transforms::pass_builder::LLVMPassBuilderOptionsRef;

use llvm_sys::transforms::pass_builder;

#[no_mangle]
pub unsafe extern "C" fn LLVMRunPasses(
    M: LLVMModuleRef,
    Passes: *const ::libc::c_char,
    TM: LLVMTargetMachineRef,
    Options: LLVMPassBuilderOptionsRef,
) -> LLVMErrorRef {
    pass_builder::LLVMRunPasses(M, Passes, TM, Options)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMCreatePassBuilderOptions() -> LLVMPassBuilderOptionsRef {
    pass_builder::LLVMCreatePassBuilderOptions()
}
#[no_mangle]
pub unsafe extern "C" fn LLVMPassBuilderOptionsSetVerifyEach(
    Options: LLVMPassBuilderOptionsRef,
    VerifyEach: LLVMBool,
) {
    pass_builder::LLVMPassBuilderOptionsSetVerifyEach(Options, VerifyEach)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMPassBuilderOptionsSetDebugLogging(
    Options: LLVMPassBuilderOptionsRef,
    DebugLogging: LLVMBool,
) {
    pass_builder::LLVMPassBuilderOptionsSetDebugLogging(Options, DebugLogging)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMPassBuilderOptionsSetLoopInterleaving(
    Options: LLVMPassBuilderOptionsRef,
    LoopInterleaving: LLVMBool,
) {
    pass_builder::LLVMPassBuilderOptionsSetLoopInterleaving(Options, LoopInterleaving)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMPassBuilderOptionsSetLoopVectorization(
    Options: LLVMPassBuilderOptionsRef,
    LoopVectorization: LLVMBool,
) {
    pass_builder::LLVMPassBuilderOptionsSetLoopVectorization(Options, LoopVectorization)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMPassBuilderOptionsSetSLPVectorization(
    Options: LLVMPassBuilderOptionsRef,
    SLPVectorization: LLVMBool,
) {
    pass_builder::LLVMPassBuilderOptionsSetSLPVectorization(Options, SLPVectorization)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMPassBuilderOptionsSetLoopUnrolling(
    Options: LLVMPassBuilderOptionsRef,
    LoopUnrolling: LLVMBool,
) {
    pass_builder::LLVMPassBuilderOptionsSetLoopUnrolling(Options, LoopUnrolling)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMPassBuilderOptionsSetForgetAllSCEVInLoopUnroll(
    Options: LLVMPassBuilderOptionsRef,
    ForgetAllSCEVInLoopUnroll: LLVMBool,
) {
    pass_builder::LLVMPassBuilderOptionsSetForgetAllSCEVInLoopUnroll(
        Options,
        ForgetAllSCEVInLoopUnroll,
    )
}
#[no_mangle]
pub unsafe extern "C" fn LLVMPassBuilderOptionsSetLicmMssaOptCap(
    Options: LLVMPassBuilderOptionsRef,
    LicmMssaOptCap: ::libc::c_uint,
) {
    pass_builder::LLVMPassBuilderOptionsSetLicmMssaOptCap(Options, LicmMssaOptCap)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMPassBuilderOptionsSetLicmMssaNoAccForPromotionCap(
    Options: LLVMPassBuilderOptionsRef,
    LicmMssaNoAccForPromotionCap: ::libc::c_uint,
) {
    pass_builder::LLVMPassBuilderOptionsSetLicmMssaNoAccForPromotionCap(
        Options,
        LicmMssaNoAccForPromotionCap,
    )
}
#[no_mangle]
pub unsafe extern "C" fn LLVMPassBuilderOptionsSetCallGraphProfile(
    Options: LLVMPassBuilderOptionsRef,
    CallGraphProfile: LLVMBool,
) {
    pass_builder::LLVMPassBuilderOptionsSetCallGraphProfile(Options, CallGraphProfile)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMPassBuilderOptionsSetMergeFunctions(
    Options: LLVMPassBuilderOptionsRef,
    MergeFunctions: LLVMBool,
) {
    pass_builder::LLVMPassBuilderOptionsSetMergeFunctions(Options, MergeFunctions)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMDisposePassBuilderOptions(Options: LLVMPassBuilderOptionsRef) {
    pass_builder::LLVMDisposePassBuilderOptions(Options)
}
