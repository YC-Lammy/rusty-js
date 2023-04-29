//! Scalar transformations of LLVM IR.

use llvm_sys::prelude::*;

use llvm_sys::transforms::scalar;

#[no_mangle]
pub unsafe extern "C" fn LLVMAddAggressiveDCEPass(PM: LLVMPassManagerRef) {
    scalar::LLVMAddAggressiveDCEPass(PM)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMAddDCEPass(PM: LLVMPassManagerRef) {
    scalar::LLVMAddDCEPass(PM)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMAddBitTrackingDCEPass(PM: LLVMPassManagerRef) {
    scalar::LLVMAddBitTrackingDCEPass(PM)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMAddAlignmentFromAssumptionsPass(PM: LLVMPassManagerRef) {
    scalar::LLVMAddAlignmentFromAssumptionsPass(PM)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMAddCFGSimplificationPass(PM: LLVMPassManagerRef) {
    scalar::LLVMAddCFGSimplificationPass(PM)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMAddDeadStoreEliminationPass(PM: LLVMPassManagerRef) {
    scalar::LLVMAddDeadStoreEliminationPass(PM)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMAddScalarizerPass(PM: LLVMPassManagerRef) {
    scalar::LLVMAddScalarizerPass(PM)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMAddMergedLoadStoreMotionPass(PM: LLVMPassManagerRef) {
    scalar::LLVMAddMergedLoadStoreMotionPass(PM)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMAddGVNPass(PM: LLVMPassManagerRef) {
    scalar::LLVMAddGVNPass(PM)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMAddNewGVNPass(PM: LLVMPassManagerRef) {
    scalar::LLVMAddNewGVNPass(PM)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMAddIndVarSimplifyPass(PM: LLVMPassManagerRef) {
    scalar::LLVMAddIndVarSimplifyPass(PM)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMAddInstructionCombiningPass(PM: LLVMPassManagerRef) {
    scalar::LLVMAddInstructionCombiningPass(PM)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMAddInstructionSimplifyPass(PM: LLVMPassManagerRef) {
    scalar::LLVMAddInstructionSimplifyPass(PM)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMAddJumpThreadingPass(PM: LLVMPassManagerRef) {
    scalar::LLVMAddJumpThreadingPass(PM)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMAddLICMPass(PM: LLVMPassManagerRef) {
    scalar::LLVMAddLICMPass(PM)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMAddLoopDeletionPass(PM: LLVMPassManagerRef) {
    scalar::LLVMAddLoopDeletionPass(PM)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMAddLoopIdiomPass(PM: LLVMPassManagerRef) {
    scalar::LLVMAddLoopIdiomPass(PM)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMAddLoopRotatePass(PM: LLVMPassManagerRef) {
    scalar::LLVMAddLoopRotatePass(PM)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMAddLoopRerollPass(PM: LLVMPassManagerRef) {
    scalar::LLVMAddLoopRerollPass(PM)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMAddLoopUnrollPass(PM: LLVMPassManagerRef) {
    scalar::LLVMAddLoopUnrollPass(PM)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMAddLoopUnrollAndJamPass(PM: LLVMPassManagerRef) {
    scalar::LLVMAddLoopUnrollAndJamPass(PM)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMAddLoopUnswitchPass(PM: LLVMPassManagerRef) {
    scalar::LLVMAddLoopUnswitchPass(PM)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMAddLowerAtomicPass(PM: LLVMPassManagerRef) {
    scalar::LLVMAddLowerAtomicPass(PM)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMAddMemCpyOptPass(PM: LLVMPassManagerRef) {
    scalar::LLVMAddMemCpyOptPass(PM)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMAddPartiallyInlineLibCallsPass(PM: LLVMPassManagerRef) {
    scalar::LLVMAddPartiallyInlineLibCallsPass(PM)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMAddReassociatePass(PM: LLVMPassManagerRef) {
    scalar::LLVMAddReassociatePass(PM)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMAddSCCPPass(PM: LLVMPassManagerRef) {
    scalar::LLVMAddSCCPPass(PM)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMAddScalarReplAggregatesPass(PM: LLVMPassManagerRef) {
    scalar::LLVMAddScalarReplAggregatesPass(PM)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMAddScalarReplAggregatesPassSSA(PM: LLVMPassManagerRef) {
    scalar::LLVMAddScalarReplAggregatesPassSSA(PM)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMAddScalarReplAggregatesPassWithThreshold(
    PM: LLVMPassManagerRef,
    Threshold: ::libc::c_int,
) {
    scalar::LLVMAddScalarReplAggregatesPassWithThreshold(PM, Threshold)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMAddSimplifyLibCallsPass(PM: LLVMPassManagerRef) {
    scalar::LLVMAddSimplifyLibCallsPass(PM)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMAddTailCallEliminationPass(PM: LLVMPassManagerRef) {
    scalar::LLVMAddTailCallEliminationPass(PM)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMAddDemoteMemoryToRegisterPass(PM: LLVMPassManagerRef) {
    scalar::LLVMAddDemoteMemoryToRegisterPass(PM)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMAddVerifierPass(PM: LLVMPassManagerRef) {
    scalar::LLVMAddVerifierPass(PM)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMAddCorrelatedValuePropagationPass(PM: LLVMPassManagerRef) {
    scalar::LLVMAddCorrelatedValuePropagationPass(PM)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMAddEarlyCSEPass(PM: LLVMPassManagerRef) {
    scalar::LLVMAddEarlyCSEPass(PM)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMAddEarlyCSEMemSSAPass(PM: LLVMPassManagerRef) {
    scalar::LLVMAddEarlyCSEMemSSAPass(PM)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMAddLowerExpectIntrinsicPass(PM: LLVMPassManagerRef) {
    scalar::LLVMAddLowerExpectIntrinsicPass(PM)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMAddLowerConstantIntrinsicsPass(PM: LLVMPassManagerRef) {
    scalar::LLVMAddLowerConstantIntrinsicsPass(PM)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMAddTypeBasedAliasAnalysisPass(PM: LLVMPassManagerRef) {
    scalar::LLVMAddTypeBasedAliasAnalysisPass(PM)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMAddScopedNoAliasAAPass(PM: LLVMPassManagerRef) {
    scalar::LLVMAddScopedNoAliasAAPass(PM)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMAddBasicAliasAnalysisPass(PM: LLVMPassManagerRef) {
    scalar::LLVMAddBasicAliasAnalysisPass(PM)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMAddUnifyFunctionExitNodesPass(PM: LLVMPassManagerRef) {
    scalar::LLVMAddUnifyFunctionExitNodesPass(PM)
}
