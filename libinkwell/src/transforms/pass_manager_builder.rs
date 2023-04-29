use llvm_sys::prelude::*;

pub use llvm_sys::transforms::pass_manager_builder::{
    LLVMOpaquePassManagerBuilder, LLVMPassManagerBuilderRef,
};

use llvm_sys::transforms::pass_manager_builder;

#[no_mangle]
pub unsafe extern "C" fn LLVMPassManagerBuilderCreate() -> LLVMPassManagerBuilderRef {
    pass_manager_builder::LLVMPassManagerBuilderCreate()
}
#[no_mangle]
pub unsafe extern "C" fn LLVMPassManagerBuilderDispose(PMB: LLVMPassManagerBuilderRef) {
    pass_manager_builder::LLVMPassManagerBuilderDispose(PMB)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMPassManagerBuilderSetOptLevel(
    PMB: LLVMPassManagerBuilderRef,
    OptLevel: ::libc::c_uint,
) {
    pass_manager_builder::LLVMPassManagerBuilderSetOptLevel(PMB, OptLevel)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMPassManagerBuilderSetSizeLevel(
    PMB: LLVMPassManagerBuilderRef,
    SizeLevel: ::libc::c_uint,
) {
    pass_manager_builder::LLVMPassManagerBuilderSetSizeLevel(PMB, SizeLevel)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMPassManagerBuilderSetDisableUnitAtATime(
    PMB: LLVMPassManagerBuilderRef,
    Value: LLVMBool,
) {
    pass_manager_builder::LLVMPassManagerBuilderSetDisableUnitAtATime(PMB, Value)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMPassManagerBuilderSetDisableUnrollLoops(
    PMB: LLVMPassManagerBuilderRef,
    Value: LLVMBool,
) {
    pass_manager_builder::LLVMPassManagerBuilderSetDisableUnrollLoops(PMB, Value)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMPassManagerBuilderSetDisableSimplifyLibCalls(
    PMB: LLVMPassManagerBuilderRef,
    Value: LLVMBool,
) {
    pass_manager_builder::LLVMPassManagerBuilderSetDisableSimplifyLibCalls(PMB, Value)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMPassManagerBuilderUseInlinerWithThreshold(
    PMB: LLVMPassManagerBuilderRef,
    Threshold: ::libc::c_uint,
) {
    pass_manager_builder::LLVMPassManagerBuilderUseInlinerWithThreshold(PMB, Threshold)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMPassManagerBuilderPopulateFunctionPassManager(
    PMB: LLVMPassManagerBuilderRef,
    PM: LLVMPassManagerRef,
) {
    pass_manager_builder::LLVMPassManagerBuilderPopulateFunctionPassManager(PMB, PM)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMPassManagerBuilderPopulateModulePassManager(
    PMB: LLVMPassManagerBuilderRef,
    PM: LLVMPassManagerRef,
) {
    pass_manager_builder::LLVMPassManagerBuilderPopulateModulePassManager(PMB, PM)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMPassManagerBuilderPopulateLTOPassManager(
    PMB: LLVMPassManagerBuilderRef,
    PM: LLVMPassManagerRef,
    Internalize: LLVMBool,
    RunInliner: LLVMBool,
) {
    pass_manager_builder::LLVMPassManagerBuilderPopulateLTOPassManager(
        PMB,
        PM,
        Internalize,
        RunInliner,
    )
}
