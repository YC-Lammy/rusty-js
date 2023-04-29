use super::*;

#[no_mangle]
pub unsafe extern "C" fn LLVMOrcCreateRTDyldObjectLinkingLayerWithSectionMemoryManager(
    ES: LLVMOrcExecutionSessionRef,
) -> LLVMOrcObjectLayerRef {
    ee::LLVMOrcCreateRTDyldObjectLinkingLayerWithSectionMemoryManager(ES)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMOrcRTDyldObjectLinkingLayerRegisterJITEventListener(
    RTDyldObjLinkingLayer: LLVMOrcObjectLayerRef,
    Listener: LLVMJITEventListenerRef,
) {
    ee::LLVMOrcRTDyldObjectLinkingLayerRegisterJITEventListener(RTDyldObjLinkingLayer, Listener)
}
