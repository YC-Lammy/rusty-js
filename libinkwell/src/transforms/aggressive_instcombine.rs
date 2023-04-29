use llvm_sys::prelude::*;

#[no_mangle]
pub unsafe extern "C" fn LLVMAddAggressiveInstCombinerPass(PM: LLVMPassManagerRef) {
    llvm_sys::transforms::aggressive_instcombine::LLVMAddAggressiveInstCombinerPass(PM)
}
