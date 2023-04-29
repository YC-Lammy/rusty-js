use llvm_sys::prelude::*;
use llvm_sys::transforms::pass_manager_builder::LLVMPassManagerBuilderRef;

#[no_mangle]
pub unsafe extern "C" fn LLVMAddCoroEarlyPass(PM: LLVMPassManagerRef) {
    llvm_sys::transforms::coroutines::LLVMAddCoroEarlyPass(PM)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMAddCoroSplitPass(PM: LLVMPassManagerRef) {
    llvm_sys::transforms::coroutines::LLVMAddCoroSplitPass(PM)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMAddCoroElidePass(PM: LLVMPassManagerRef) {
    llvm_sys::transforms::coroutines::LLVMAddCoroElidePass(PM)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMAddCoroCleanupPass(PM: LLVMPassManagerRef) {
    llvm_sys::transforms::coroutines::LLVMAddCoroCleanupPass(PM)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMPassManagerBuilderAddCoroutinePassesToExtensionPoints(
    PMB: LLVMPassManagerBuilderRef,
) {
    llvm_sys::transforms::coroutines::LLVMPassManagerBuilderAddCoroutinePassesToExtensionPoints(PMB)
}
