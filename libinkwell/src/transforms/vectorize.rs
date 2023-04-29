//! Vectorization transformations of LLVM IR.

use llvm_sys::prelude::*;

#[no_mangle]
pub unsafe extern "C" fn LLVMAddLoopVectorizePass(PM: LLVMPassManagerRef) {
    llvm_sys::transforms::vectorize::LLVMAddLoopVectorizePass(PM)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMAddSLPVectorizePass(PM: LLVMPassManagerRef) {
    llvm_sys::transforms::vectorize::LLVMAddSLPVectorizePass(PM)
}
