use llvm_sys::prelude::*;

#[no_mangle]
pub unsafe extern "C" fn LLVMAddLowerSwitchPass(PM: LLVMPassManagerRef) {
    llvm_sys::transforms::util::LLVMAddLowerSwitchPass(PM)
}

#[no_mangle]
pub unsafe extern "C" fn LLVMAddPromoteMemoryToRegisterPass(PM: LLVMPassManagerRef) {
    llvm_sys::transforms::util::LLVMAddPromoteMemoryToRegisterPass(PM)
}

#[no_mangle]
pub unsafe extern "C" fn LLVMAddAddDiscriminatorsPass(PM: LLVMPassManagerRef) {
    llvm_sys::transforms::util::LLVMAddAddDiscriminatorsPass(PM)
}
