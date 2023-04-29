//! Interprocedural transformations of LLVM IR.

use llvm_sys::prelude::*;

#[no_mangle]
pub unsafe extern "C" fn LLVMAddArgumentPromotionPass(PM: LLVMPassManagerRef) {
    llvm_sys::transforms::ipo::LLVMAddArgumentPromotionPass(PM)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMAddConstantMergePass(PM: LLVMPassManagerRef) {
    llvm_sys::transforms::ipo::LLVMAddConstantMergePass(PM)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMAddMergeFunctionsPass(PM: LLVMPassManagerRef) {
    llvm_sys::transforms::ipo::LLVMAddMergeFunctionsPass(PM)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMAddCalledValuePropagationPass(PM: LLVMPassManagerRef) {
    llvm_sys::transforms::ipo::LLVMAddCalledValuePropagationPass(PM)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMAddDeadArgEliminationPass(PM: LLVMPassManagerRef) {
    llvm_sys::transforms::ipo::LLVMAddDeadArgEliminationPass(PM)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMAddFunctionAttrsPass(PM: LLVMPassManagerRef) {
    llvm_sys::transforms::ipo::LLVMAddFunctionAttrsPass(PM)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMAddFunctionInliningPass(PM: LLVMPassManagerRef) {
    llvm_sys::transforms::ipo::LLVMAddFunctionInliningPass(PM)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMAddAlwaysInlinerPass(PM: LLVMPassManagerRef) {
    llvm_sys::transforms::ipo::LLVMAddAlwaysInlinerPass(PM)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMAddGlobalDCEPass(PM: LLVMPassManagerRef) {
    llvm_sys::transforms::ipo::LLVMAddGlobalDCEPass(PM)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMAddGlobalOptimizerPass(PM: LLVMPassManagerRef) {
    llvm_sys::transforms::ipo::LLVMAddGlobalOptimizerPass(PM)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMAddPruneEHPass(PM: LLVMPassManagerRef) {
    llvm_sys::transforms::ipo::LLVMAddPruneEHPass(PM)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMAddIPSCCPPass(PM: LLVMPassManagerRef) {
    llvm_sys::transforms::ipo::LLVMAddIPSCCPPass(PM)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMAddInternalizePass(
    arg1: LLVMPassManagerRef,
    AllButMain: ::libc::c_uint,
) {
    llvm_sys::transforms::ipo::LLVMAddInternalizePass(arg1, AllButMain)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMAddInternalizePassWithMustPreservePredicate(
    PM: LLVMPassManagerRef,
    Context: *mut ::libc::c_void,
    MustPreserve: Option<extern "C" fn(LLVMValueRef, *mut ::libc::c_void) -> LLVMBool>,
) {
    llvm_sys::transforms::ipo::LLVMAddInternalizePassWithMustPreservePredicate(
        PM,
        Context,
        MustPreserve,
    )
}
#[no_mangle]
pub unsafe extern "C" fn LLVMAddStripDeadPrototypesPass(PM: LLVMPassManagerRef) {
    llvm_sys::transforms::ipo::LLVMAddStripDeadPrototypesPass(PM)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMAddStripSymbolsPass(PM: LLVMPassManagerRef) {
    llvm_sys::transforms::ipo::LLVMAddStripSymbolsPass(PM)
}
