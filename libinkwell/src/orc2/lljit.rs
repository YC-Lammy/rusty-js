use super::*;
use llvm_sys::error::LLVMErrorRef;
use llvm_sys::prelude::*;

pub use llvm_sys::orc2::lljit::{
    LLVMOrcLLJITBuilderObjectLinkingLayerCreatorFunction, LLVMOrcLLJITBuilderRef, LLVMOrcLLJITRef,
    LLVMOrcOpaqueLLJIT, LLVMOrcOpaqueLLJITBuilder,
};

use llvm_sys::orc2::lljit;

#[no_mangle]
pub unsafe extern "C" fn LLVMOrcCreateLLJITBuilder() -> LLVMOrcLLJITBuilderRef {
    lljit::LLVMOrcCreateLLJITBuilder()
}
#[no_mangle]
pub unsafe extern "C" fn LLVMOrcDisposeLLJITBuilder(Builder: LLVMOrcLLJITBuilderRef) {
    lljit::LLVMOrcDisposeLLJITBuilder(Builder)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMOrcLLJITBuilderSetJITTargetMachineBuilder(
    Builder: LLVMOrcLLJITBuilderRef,
    JTMB: LLVMOrcJITTargetMachineBuilderRef,
) {
    lljit::LLVMOrcLLJITBuilderSetJITTargetMachineBuilder(Builder, JTMB)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMOrcLLJITBuilderSetObjectLinkingLayerCreator(
    Builder: LLVMOrcLLJITBuilderRef,
    F: LLVMOrcLLJITBuilderObjectLinkingLayerCreatorFunction,
    Ctx: *mut ::libc::c_void,
) {
    lljit::LLVMOrcLLJITBuilderSetObjectLinkingLayerCreator(Builder, F, Ctx)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMOrcCreateLLJIT(
    Result: *mut LLVMOrcLLJITRef,
    Builder: LLVMOrcLLJITBuilderRef,
) -> LLVMErrorRef {
    lljit::LLVMOrcCreateLLJIT(Result, Builder)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMOrcDisposeLLJIT(J: LLVMOrcLLJITRef) -> LLVMErrorRef {
    lljit::LLVMOrcDisposeLLJIT(J)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMOrcLLJITGetExecutionSession(
    J: LLVMOrcLLJITRef,
) -> LLVMOrcExecutionSessionRef {
    lljit::LLVMOrcLLJITGetExecutionSession(J)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMOrcLLJITGetMainJITDylib(J: LLVMOrcLLJITRef) -> LLVMOrcJITDylibRef {
    lljit::LLVMOrcLLJITGetMainJITDylib(J)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMOrcLLJITGetTripleString(J: LLVMOrcLLJITRef) -> *const ::libc::c_char {
    lljit::LLVMOrcLLJITGetTripleString(J)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMOrcLLJITGetGlobalPrefix(J: LLVMOrcLLJITRef) -> ::libc::c_char {
    lljit::LLVMOrcLLJITGetGlobalPrefix(J)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMOrcLLJITMangleAndIntern(
    J: LLVMOrcLLJITRef,
    UnmangledName: *const ::libc::c_char,
) -> LLVMOrcSymbolStringPoolEntryRef {
    lljit::LLVMOrcLLJITMangleAndIntern(J, UnmangledName)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMOrcLLJITAddObjectFile(
    J: LLVMOrcLLJITRef,
    JD: LLVMOrcJITDylibRef,
    ObjBuffer: LLVMMemoryBufferRef,
) -> LLVMErrorRef {
    lljit::LLVMOrcLLJITAddObjectFile(J, JD, ObjBuffer)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMOrcLLJITAddObjectFileWithRT(
    J: LLVMOrcLLJITRef,
    RT: LLVMOrcResourceTrackerRef,
    ObjBuffer: LLVMMemoryBufferRef,
) -> LLVMErrorRef {
    lljit::LLVMOrcLLJITAddObjectFileWithRT(J, RT, ObjBuffer)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMOrcLLJITAddLLVMIRModule(
    J: LLVMOrcLLJITRef,
    JD: LLVMOrcJITDylibRef,
    TSM: LLVMOrcThreadSafeModuleRef,
) -> LLVMErrorRef {
    lljit::LLVMOrcLLJITAddLLVMIRModule(J, JD, TSM)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMOrcLLJITAddLLVMIRModuleWithRT(
    J: LLVMOrcLLJITRef,
    JD: LLVMOrcResourceTrackerRef,
    TSM: LLVMOrcThreadSafeModuleRef,
) -> LLVMErrorRef {
    lljit::LLVMOrcLLJITAddLLVMIRModuleWithRT(J, JD, TSM)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMOrcLLJITLookup(
    J: LLVMOrcLLJITRef,
    Result: *mut LLVMOrcExecutorAddress,
    Name: *const ::libc::c_char,
) -> LLVMErrorRef {
    lljit::LLVMOrcLLJITLookup(J, Result, Name)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMOrcLLJITGetObjLinkingLayer(
    J: LLVMOrcLLJITRef,
) -> LLVMOrcObjectLayerRef {
    lljit::LLVMOrcLLJITGetObjLinkingLayer(J)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMOrcLLJITGetObjTransformLayer(
    J: LLVMOrcLLJITRef,
) -> LLVMOrcObjectTransformLayerRef {
    lljit::LLVMOrcLLJITGetObjTransformLayer(J)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMOrcLLJITGetIRTransformLayer(
    J: LLVMOrcLLJITRef,
) -> LLVMOrcIRTransformLayerRef {
    lljit::LLVMOrcLLJITGetIRTransformLayer(J)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMOrcLLJITGetDataLayoutStr(J: LLVMOrcLLJITRef) -> *const ::libc::c_char {
    lljit::LLVMOrcLLJITGetDataLayoutStr(J)
}
