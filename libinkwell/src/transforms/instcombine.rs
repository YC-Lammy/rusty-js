use llvm_sys::prelude::*;

#[no_mangle]
pub unsafe extern "C" fn LLVMINSTAddInstructionCombiningPass(PM: LLVMPassManagerRef) {
    llvm_sys::transforms::instcombine::LLVMAddInstructionCombiningPass(PM)
}
