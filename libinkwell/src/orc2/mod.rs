#![allow(non_snake_case)]
//! OrcV2

pub mod ee;
pub mod lljit;

use llvm_sys::error::LLVMErrorRef;
use llvm_sys::prelude::*;
use llvm_sys::target_machine::LLVMTargetMachineRef;

pub use llvm_sys::orc2::*;

use llvm_sys::orc2;

#[no_mangle]
pub unsafe extern "C" fn LLVMOrcExecutionSessionSetErrorReporter(
    ES: LLVMOrcExecutionSessionRef,
    ReportError: LLVMOrcErrorReporterFunction,
    Ctx: *mut ::libc::c_void,
) {
    orc2::LLVMOrcExecutionSessionSetErrorReporter(ES, ReportError, Ctx)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMOrcExecutionSessionGetSymbolStringPool(
    ES: LLVMOrcExecutionSessionRef,
) -> LLVMOrcSymbolStringPoolRef {
    orc2::LLVMOrcExecutionSessionGetSymbolStringPool(ES)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMOrcSymbolStringPoolClearDeadEntries(SSP: LLVMOrcSymbolStringPoolRef) {
    orc2::LLVMOrcSymbolStringPoolClearDeadEntries(SSP)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMOrcExecutionSessionIntern(
    ES: LLVMOrcExecutionSessionRef,
    Name: *const ::libc::c_char,
) -> LLVMOrcSymbolStringPoolEntryRef {
    orc2::LLVMOrcExecutionSessionIntern(ES, Name)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMOrcRetainSymbolStringPoolEntry(S: LLVMOrcSymbolStringPoolEntryRef) {
    orc2::LLVMOrcRetainSymbolStringPoolEntry(S)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMOrcReleaseSymbolStringPoolEntry(S: LLVMOrcSymbolStringPoolEntryRef) {
    orc2::LLVMOrcReleaseSymbolStringPoolEntry(S)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMOrcSymbolStringPoolEntryStr(
    S: LLVMOrcSymbolStringPoolEntryRef,
) -> *const ::libc::c_char {
    orc2::LLVMOrcSymbolStringPoolEntryStr(S)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMOrcReleaseResourceTracker(RT: LLVMOrcResourceTrackerRef) {
    orc2::LLVMOrcReleaseResourceTracker(RT)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMOrcResourceTrackerTransferTo(
    SrcRT: LLVMOrcResourceTrackerRef,
    DstRT: LLVMOrcResourceTrackerRef,
) {
    orc2::LLVMOrcResourceTrackerTransferTo(SrcRT, DstRT)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMOrcResourceTrackerRemove(
    RT: LLVMOrcResourceTrackerRef,
) -> LLVMErrorRef {
    orc2::LLVMOrcResourceTrackerRemove(RT)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMOrcDisposeDefinitionGenerator(DG: LLVMOrcDefinitionGeneratorRef) {
    orc2::LLVMOrcDisposeDefinitionGenerator(DG)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMOrcDisposeMaterializationUnit(MU: LLVMOrcMaterializationUnitRef) {
    orc2::LLVMOrcDisposeMaterializationUnit(MU)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMOrcCreateCustomMaterializationUnit(
    Name: *const ::libc::c_char,
    Ctx: *mut ::libc::c_void,
    Syms: LLVMOrcCSymbolFlagsMapPairs,
    NumSyms: ::libc::size_t,
    InitSym: LLVMOrcSymbolStringPoolEntryRef,
    Materialize: LLVMOrcMaterializationUnitMaterializeFunction,
    Discard: LLVMOrcMaterializationUnitDiscardFunction,
    Destroy: LLVMOrcMaterializationUnitDestroyFunction,
) -> LLVMOrcMaterializationUnitRef {
    orc2::LLVMOrcCreateCustomMaterializationUnit(
        Name,
        Ctx,
        Syms,
        NumSyms,
        InitSym,
        Materialize,
        Discard,
        Destroy,
    )
}
#[no_mangle]
pub unsafe extern "C" fn LLVMOrcAbsoluteSymbols(
    Syms: LLVMOrcCSymbolMapPairs,
    NumPairs: usize,
) -> LLVMOrcMaterializationUnitRef {
    orc2::LLVMOrcAbsoluteSymbols(Syms, NumPairs)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMOrcLazyReexports(
    LCTM: LLVMOrcLazyCallThroughManagerRef,
    ISM: LLVMOrcIndirectStubsManagerRef,
    SourceRef: LLVMOrcJITDylibRef,
    CallableAliases: LLVMOrcCSymbolAliasMapPairs,
    NumPairs: ::libc::size_t,
) -> LLVMOrcMaterializationUnitRef {
    orc2::LLVMOrcLazyReexports(LCTM, ISM, SourceRef, CallableAliases, NumPairs)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMOrcDisposeMaterializationResponsibility(
    MR: LLVMOrcMaterializationResponsibilityRef,
) {
    orc2::LLVMOrcDisposeMaterializationResponsibility(MR)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMOrcMaterializationResponsibilityGetTargetDylib(
    MR: LLVMOrcMaterializationResponsibilityRef,
) -> LLVMOrcJITDylibRef {
    orc2::LLVMOrcMaterializationResponsibilityGetTargetDylib(MR)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMOrcMaterializationResponsibilityGetExecutionSession(
    MR: LLVMOrcMaterializationResponsibilityRef,
) -> LLVMOrcExecutionSessionRef {
    orc2::LLVMOrcMaterializationResponsibilityGetExecutionSession(MR)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMOrcMaterializationResponsibilityGetSymbols(
    MR: LLVMOrcMaterializationResponsibilityRef,
    NumPairs: *mut ::libc::size_t,
) -> LLVMOrcCSymbolFlagsMapPairs {
    orc2::LLVMOrcMaterializationResponsibilityGetSymbols(MR, NumPairs)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMOrcDisposeCSymbolFlagsMap(Pairs: LLVMOrcCSymbolFlagsMapPairs) {
    orc2::LLVMOrcDisposeCSymbolFlagsMap(Pairs)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMOrcMaterializationResponsibilityGetInitializerSymbol(
    MR: LLVMOrcMaterializationResponsibilityRef,
) -> LLVMOrcSymbolStringPoolEntryRef {
    orc2::LLVMOrcMaterializationResponsibilityGetInitializerSymbol(MR)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMOrcMaterializationResponsibilityGetRequestedSymbols(
    MR: LLVMOrcMaterializationResponsibilityRef,
    NumSymbols: *mut ::libc::size_t,
) -> *mut LLVMOrcSymbolStringPoolEntryRef {
    orc2::LLVMOrcMaterializationResponsibilityGetRequestedSymbols(MR, NumSymbols)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMOrcDisposeSymbols(Symbols: *mut LLVMOrcSymbolStringPoolEntryRef) {
    orc2::LLVMOrcDisposeSymbols(Symbols)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMOrcMaterializationResponsibilityNotifyResolved(
    MR: LLVMOrcMaterializationResponsibilityRef,
    Symbols: LLVMOrcCSymbolMapPairs,
    NumPairs: ::libc::size_t,
) -> LLVMErrorRef {
    orc2::LLVMOrcMaterializationResponsibilityNotifyResolved(MR, Symbols, NumPairs)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMOrcMaterializationResponsibilityNotifyEmitted(
    MR: LLVMOrcMaterializationResponsibilityRef,
) -> LLVMErrorRef {
    orc2::LLVMOrcMaterializationResponsibilityNotifyEmitted(MR)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMOrcMaterializationResponsibilityDefineMaterializing(
    MR: LLVMOrcMaterializationResponsibilityRef,
    Pairs: LLVMOrcCSymbolFlagsMapPairs,
    NumPairs: ::libc::size_t,
) -> LLVMErrorRef {
    orc2::LLVMOrcMaterializationResponsibilityDefineMaterializing(MR, Pairs, NumPairs)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMOrcMaterializationResponsibilityFailMaterialization(
    MR: LLVMOrcMaterializationResponsibilityRef,
) {
    orc2::LLVMOrcMaterializationResponsibilityFailMaterialization(MR)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMOrcMaterializationResponsibilityReplace(
    MR: LLVMOrcMaterializationResponsibilityRef,
    MU: LLVMOrcMaterializationUnitRef,
) -> LLVMErrorRef {
    orc2::LLVMOrcMaterializationResponsibilityReplace(MR, MU)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMOrcMaterializationResponsibilityDelegate(
    MR: LLVMOrcMaterializationResponsibilityRef,
    Symbols: *mut LLVMOrcSymbolStringPoolEntryRef,
    NumSymbols: ::libc::size_t,
    Result: *mut LLVMOrcMaterializationResponsibilityRef,
) -> LLVMErrorRef {
    orc2::LLVMOrcMaterializationResponsibilityDelegate(MR, Symbols, NumSymbols, Result)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMOrcMaterializationResponsibilityAddDependencies(
    MR: LLVMOrcMaterializationResponsibilityRef,
    Name: LLVMOrcSymbolStringPoolEntryRef,
    Dependencies: LLVMOrcCDependenceMapPairs,
    NumPairs: ::libc::size_t,
) {
    orc2::LLVMOrcMaterializationResponsibilityAddDependencies(MR, Name, Dependencies, NumPairs)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMOrcMaterializationResponsibilityAddDependenciesForAll(
    MR: LLVMOrcMaterializationResponsibilityRef,
    Dependencies: LLVMOrcCDependenceMapPairs,
    NumPairs: ::libc::size_t,
) {
    orc2::LLVMOrcMaterializationResponsibilityAddDependenciesForAll(MR, Dependencies, NumPairs)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMOrcExecutionSessionCreateBareJITDylib(
    ES: LLVMOrcExecutionSessionRef,
    Name: *const ::libc::c_char,
) -> LLVMOrcJITDylibRef {
    orc2::LLVMOrcExecutionSessionCreateBareJITDylib(ES, Name)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMOrcExecutionSessionCreateJITDylib(
    ES: LLVMOrcExecutionSessionRef,
    Result_: *mut LLVMOrcJITDylibRef,
    Name: *const ::libc::c_char,
) -> LLVMErrorRef {
    orc2::LLVMOrcExecutionSessionCreateJITDylib(ES, Result_, Name)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMOrcExecutionSessionGetJITDylibByName(
    ES: LLVMOrcExecutionSessionRef,
    Name: *const ::libc::c_char,
) -> LLVMOrcJITDylibRef {
    orc2::LLVMOrcExecutionSessionGetJITDylibByName(ES, Name)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMOrcJITDylibCreateResourceTracker(
    JD: LLVMOrcJITDylibRef,
) -> LLVMOrcResourceTrackerRef {
    orc2::LLVMOrcJITDylibCreateResourceTracker(JD)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMOrcJITDylibGetDefaultResourceTracker(
    JD: LLVMOrcJITDylibRef,
) -> LLVMOrcResourceTrackerRef {
    orc2::LLVMOrcJITDylibGetDefaultResourceTracker(JD)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMOrcJITDylibDefine(
    JD: LLVMOrcJITDylibRef,
    MU: LLVMOrcMaterializationUnitRef,
) -> LLVMErrorRef {
    orc2::LLVMOrcJITDylibDefine(JD, MU)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMOrcJITDylibClear(JD: LLVMOrcJITDylibRef) -> LLVMErrorRef {
    orc2::LLVMOrcJITDylibClear(JD)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMOrcJITDylibAddGenerator(
    JD: LLVMOrcJITDylibRef,
    DG: LLVMOrcDefinitionGeneratorRef,
) {
    orc2::LLVMOrcJITDylibAddGenerator(JD, DG)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMOrcCreateCustomCAPIDefinitionGenerator(
    F: LLVMOrcCAPIDefinitionGeneratorTryToGenerateFunction,
    Ctx: *mut ::libc::c_void,
) -> LLVMOrcDefinitionGeneratorRef {
    orc2::LLVMOrcCreateCustomCAPIDefinitionGenerator(F, Ctx)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMOrcCreateDynamicLibrarySearchGeneratorForProcess(
    Result: *mut LLVMOrcDefinitionGeneratorRef,
    GlobalPrefix: ::libc::c_char,
    Filter: LLVMOrcSymbolPredicate,
    FilterCtx: *mut ::libc::c_void,
) -> LLVMErrorRef {
    orc2::LLVMOrcCreateDynamicLibrarySearchGeneratorForProcess(
        Result,
        GlobalPrefix,
        Filter,
        FilterCtx,
    )
}
#[no_mangle]
pub unsafe extern "C" fn LLVMOrcCreateNewThreadSafeContext() -> LLVMOrcThreadSafeContextRef {
    orc2::LLVMOrcCreateNewThreadSafeContext()
}
#[no_mangle]
pub unsafe extern "C" fn LLVMOrcThreadSafeContextGetContext(
    TSCtx: LLVMOrcThreadSafeContextRef,
) -> LLVMContextRef {
    orc2::LLVMOrcThreadSafeContextGetContext(TSCtx)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMOrcDisposeThreadSafeContext(TSCtx: LLVMOrcThreadSafeContextRef) {
    orc2::LLVMOrcDisposeThreadSafeContext(TSCtx)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMOrcCreateNewThreadSafeModule(
    M: LLVMModuleRef,
    TSCtx: LLVMOrcThreadSafeContextRef,
) -> LLVMOrcThreadSafeModuleRef {
    orc2::LLVMOrcCreateNewThreadSafeModule(M, TSCtx)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMOrcDisposeThreadSafeModule(TSM: LLVMOrcThreadSafeModuleRef) {
    orc2::LLVMOrcDisposeThreadSafeModule(TSM)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMOrcThreadSafeModuleWithModuleDo(
    TSM: LLVMOrcThreadSafeModuleRef,
    F: LLVMOrcGenericIRModuleOperationFunction,
    Ctx: *mut ::libc::c_void,
) -> LLVMErrorRef {
    orc2::LLVMOrcThreadSafeModuleWithModuleDo(TSM, F, Ctx)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMOrcJITTargetMachineBuilderDetectHost(
    Result: *mut LLVMOrcJITTargetMachineBuilderRef,
) -> LLVMErrorRef {
    orc2::LLVMOrcJITTargetMachineBuilderDetectHost(Result)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMOrcJITTargetMachineBuilderCreateFromTargetMachine(
    TM: LLVMTargetMachineRef,
) -> LLVMOrcJITTargetMachineBuilderRef {
    orc2::LLVMOrcJITTargetMachineBuilderCreateFromTargetMachine(TM)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMOrcDisposeJITTargetMachineBuilder(
    JTMB: LLVMOrcJITTargetMachineBuilderRef,
) {
    orc2::LLVMOrcDisposeJITTargetMachineBuilder(JTMB)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMOrcJITTargetMachineBuilderGetTargetTriple(
    JTMB: LLVMOrcJITTargetMachineBuilderRef,
) -> *mut ::libc::c_char {
    orc2::LLVMOrcJITTargetMachineBuilderGetTargetTriple(JTMB)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMOrcJITTargetMachineBuilderSetTargetTriple(
    JTMB: LLVMOrcJITTargetMachineBuilderRef,
    TargetTriple: *const ::libc::c_char,
) {
    orc2::LLVMOrcJITTargetMachineBuilderSetTargetTriple(JTMB, TargetTriple)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMOrcObjectLayerAddObjectFile(
    ObjLayer: LLVMOrcObjectLayerRef,
    JD: LLVMOrcJITDylibRef,
    ObjBuffer: LLVMMemoryBufferRef,
) -> LLVMErrorRef {
    orc2::LLVMOrcObjectLayerAddObjectFile(ObjLayer, JD, ObjBuffer)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMOrcObjectLayerAddObjectFileWithRT(
    ObjLayer: LLVMOrcObjectLayerRef,
    RT: LLVMOrcResourceTrackerRef,
    ObjBuffer: LLVMMemoryBufferRef,
) -> LLVMErrorRef {
    orc2::LLVMOrcObjectLayerAddObjectFileWithRT(ObjLayer, RT, ObjBuffer)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMOrcObjectLayerEmit(
    ObjLayer: LLVMOrcObjectLayerRef,
    R: LLVMOrcMaterializationResponsibilityRef,
    ObjBuffer: LLVMMemoryBufferRef,
) {
    orc2::LLVMOrcObjectLayerEmit(ObjLayer, R, ObjBuffer)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMOrcDisposeObjectLayer(ObjLayer: LLVMOrcObjectLayerRef) {
    orc2::LLVMOrcDisposeObjectLayer(ObjLayer)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMOrcIRTransformLayerEmit(
    IRTransformLayer: LLVMOrcIRTransformLayerRef,
    MR: LLVMOrcMaterializationResponsibilityRef,
    TSM: LLVMOrcThreadSafeModuleRef,
) {
    orc2::LLVMOrcIRTransformLayerEmit(IRTransformLayer, MR, TSM)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMOrcIRTransformLayerSetTransform(
    IRTransformLayer: LLVMOrcIRTransformLayerRef,
    TransformFunction: LLVMOrcIRTransformLayerTransformFunction,
    Ctx: *mut ::libc::c_void,
) {
    orc2::LLVMOrcIRTransformLayerSetTransform(IRTransformLayer, TransformFunction, Ctx)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMOrcObjectTransformLayerSetTransform(
    ObjTransformLayer: LLVMOrcObjectTransformLayerRef,
    TransformFunction: LLVMOrcObjectTransformLayerTransformFunction,
    Ctx: *mut ::libc::c_void,
) {
    orc2::LLVMOrcObjectTransformLayerSetTransform(ObjTransformLayer, TransformFunction, Ctx)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMOrcCreateLocalIndirectStubsManager(
    TargetTriple: *const ::libc::c_char,
) -> LLVMOrcIndirectStubsManagerRef {
    orc2::LLVMOrcCreateLocalIndirectStubsManager(TargetTriple)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMOrcDisposeIndirectStubsManager(ISM: LLVMOrcIndirectStubsManagerRef) {
    orc2::LLVMOrcDisposeIndirectStubsManager(ISM)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMOrcCreateLocalLazyCallThroughManager(
    TargetTriple: *const ::libc::c_char,
    ES: LLVMOrcExecutionSessionRef,
    ErrorHandlerAddr: LLVMOrcJITTargetAddress,
    LCTM: *mut LLVMOrcLazyCallThroughManagerRef,
) -> LLVMErrorRef {
    orc2::LLVMOrcCreateLocalLazyCallThroughManager(TargetTriple, ES, ErrorHandlerAddr, LCTM)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMOrcDisposeLazyCallThroughManager(
    LCTM: LLVMOrcLazyCallThroughManagerRef,
) {
    orc2::LLVMOrcDisposeLazyCallThroughManager(LCTM)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMOrcCreateDumpObjects(
    DumpDir: *const ::libc::c_char,
    IdentifierOverride: *const ::libc::c_char,
) -> LLVMOrcDumpObjectsRef {
    orc2::LLVMOrcCreateDumpObjects(DumpDir, IdentifierOverride)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMOrcDisposeDumpObjects(DumpObjects: LLVMOrcDumpObjectsRef) {
    orc2::LLVMOrcDisposeDumpObjects(DumpObjects)
}
#[no_mangle]
pub unsafe extern "C" fn LLVMOrcDumpObjects_CallOperator(
    DumpObjects: LLVMOrcDumpObjectsRef,
    ObjBuffer: *mut LLVMMemoryBufferRef,
) -> LLVMErrorRef {
    orc2::LLVMOrcDumpObjects_CallOperator(DumpObjects, ObjBuffer)
}
