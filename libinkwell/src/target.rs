# ! [doc = " Target information"] use llvm_sys :: prelude :: * ; pub use llvm_sys :: target :: * ; # [no_mangle] pub unsafe extern "C" fn LLVMInitializeAMDGPUTargetInfo () { llvm_sys :: target :: LLVMInitializeAMDGPUTargetInfo () } # [no_mangle] pub unsafe extern "C" fn LLVMInitializeAMDGPUTarget () { llvm_sys :: target :: LLVMInitializeAMDGPUTarget () } # [no_mangle] pub unsafe extern "C" fn LLVMInitializeAMDGPUTargetMC () { llvm_sys :: target :: LLVMInitializeAMDGPUTargetMC () } # [no_mangle] pub unsafe extern "C" fn LLVMInitializeAMDGPUAsmPrinter () { llvm_sys :: target :: LLVMInitializeAMDGPUAsmPrinter () } # [no_mangle] pub unsafe extern "C" fn LLVMInitializeAMDGPUAsmParser () { llvm_sys :: target :: LLVMInitializeAMDGPUAsmParser () } # [no_mangle] pub unsafe extern "C" fn LLVMInitializeSystemZTargetInfo () { llvm_sys :: target :: LLVMInitializeSystemZTargetInfo () } # [no_mangle] pub unsafe extern "C" fn LLVMInitializeSystemZTarget () { llvm_sys :: target :: LLVMInitializeSystemZTarget () } # [no_mangle] pub unsafe extern "C" fn LLVMInitializeSystemZTargetMC () { llvm_sys :: target :: LLVMInitializeSystemZTargetMC () } # [no_mangle] pub unsafe extern "C" fn LLVMInitializeSystemZAsmPrinter () { llvm_sys :: target :: LLVMInitializeSystemZAsmPrinter () } # [no_mangle] pub unsafe extern "C" fn LLVMInitializeSystemZAsmParser () { llvm_sys :: target :: LLVMInitializeSystemZAsmParser () } # [no_mangle] pub unsafe extern "C" fn LLVMInitializeSystemZDisassembler () { llvm_sys :: target :: LLVMInitializeSystemZDisassembler () } # [no_mangle] pub unsafe extern "C" fn LLVMInitializeHexagonTargetInfo () { llvm_sys :: target :: LLVMInitializeHexagonTargetInfo () } # [no_mangle] pub unsafe extern "C" fn LLVMInitializeHexagonTarget () { llvm_sys :: target :: LLVMInitializeHexagonTarget () } # [no_mangle] pub unsafe extern "C" fn LLVMInitializeHexagonTargetMC () { llvm_sys :: target :: LLVMInitializeHexagonTargetMC () } # [no_mangle] pub unsafe extern "C" fn LLVMInitializeHexagonAsmPrinter () { llvm_sys :: target :: LLVMInitializeHexagonAsmPrinter () } # [no_mangle] pub unsafe extern "C" fn LLVMInitializeHexagonDisassembler () { llvm_sys :: target :: LLVMInitializeHexagonDisassembler () } # [no_mangle] pub unsafe extern "C" fn LLVMInitializeNVPTXTargetInfo () { llvm_sys :: target :: LLVMInitializeNVPTXTargetInfo () } # [no_mangle] pub unsafe extern "C" fn LLVMInitializeNVPTXTarget () { llvm_sys :: target :: LLVMInitializeNVPTXTarget () } # [no_mangle] pub unsafe extern "C" fn LLVMInitializeNVPTXTargetMC () { llvm_sys :: target :: LLVMInitializeNVPTXTargetMC () } # [no_mangle] pub unsafe extern "C" fn LLVMInitializeNVPTXAsmPrinter () { llvm_sys :: target :: LLVMInitializeNVPTXAsmPrinter () } # [no_mangle] pub unsafe extern "C" fn LLVMInitializeMSP430TargetInfo () { llvm_sys :: target :: LLVMInitializeMSP430TargetInfo () } # [no_mangle] pub unsafe extern "C" fn LLVMInitializeMSP430Target () { llvm_sys :: target :: LLVMInitializeMSP430Target () } # [no_mangle] pub unsafe extern "C" fn LLVMInitializeMSP430TargetMC () { llvm_sys :: target :: LLVMInitializeMSP430TargetMC () } # [no_mangle] pub unsafe extern "C" fn LLVMInitializeMSP430AsmPrinter () { llvm_sys :: target :: LLVMInitializeMSP430AsmPrinter () } # [no_mangle] pub unsafe extern "C" fn LLVMInitializeXCoreTargetInfo () { llvm_sys :: target :: LLVMInitializeXCoreTargetInfo () } # [no_mangle] pub unsafe extern "C" fn LLVMInitializeXCoreTarget () { llvm_sys :: target :: LLVMInitializeXCoreTarget () } # [no_mangle] pub unsafe extern "C" fn LLVMInitializeXCoreTargetMC () { llvm_sys :: target :: LLVMInitializeXCoreTargetMC () } # [no_mangle] pub unsafe extern "C" fn LLVMInitializeXCoreAsmPrinter () { llvm_sys :: target :: LLVMInitializeXCoreAsmPrinter () } # [no_mangle] pub unsafe extern "C" fn LLVMInitializeXCoreDisassembler () { llvm_sys :: target :: LLVMInitializeXCoreDisassembler () } # [no_mangle] pub unsafe extern "C" fn LLVMInitializeMipsTargetInfo () { llvm_sys :: target :: LLVMInitializeMipsTargetInfo () } # [no_mangle] pub unsafe extern "C" fn LLVMInitializeMipsTarget () { llvm_sys :: target :: LLVMInitializeMipsTarget () } # [no_mangle] pub unsafe extern "C" fn LLVMInitializeMipsTargetMC () { llvm_sys :: target :: LLVMInitializeMipsTargetMC () } # [no_mangle] pub unsafe extern "C" fn LLVMInitializeMipsAsmPrinter () { llvm_sys :: target :: LLVMInitializeMipsAsmPrinter () } # [no_mangle] pub unsafe extern "C" fn LLVMInitializeMipsAsmParser () { llvm_sys :: target :: LLVMInitializeMipsAsmParser () } # [no_mangle] pub unsafe extern "C" fn LLVMInitializeMipsDisassembler () { llvm_sys :: target :: LLVMInitializeMipsDisassembler () } # [no_mangle] pub unsafe extern "C" fn LLVMInitializeAArch64TargetInfo () { llvm_sys :: target :: LLVMInitializeAArch64TargetInfo () } # [no_mangle] pub unsafe extern "C" fn LLVMInitializeAArch64Target () { llvm_sys :: target :: LLVMInitializeAArch64Target () } # [no_mangle] pub unsafe extern "C" fn LLVMInitializeAArch64TargetMC () { llvm_sys :: target :: LLVMInitializeAArch64TargetMC () } # [no_mangle] pub unsafe extern "C" fn LLVMInitializeAArch64AsmPrinter () { llvm_sys :: target :: LLVMInitializeAArch64AsmPrinter () } # [no_mangle] pub unsafe extern "C" fn LLVMInitializeAArch64AsmParser () { llvm_sys :: target :: LLVMInitializeAArch64AsmParser () } # [no_mangle] pub unsafe extern "C" fn LLVMInitializeAArch64Disassembler () { llvm_sys :: target :: LLVMInitializeAArch64Disassembler () } # [no_mangle] pub unsafe extern "C" fn LLVMInitializeARMTargetInfo () { llvm_sys :: target :: LLVMInitializeARMTargetInfo () } # [no_mangle] pub unsafe extern "C" fn LLVMInitializeARMTarget () { llvm_sys :: target :: LLVMInitializeARMTarget () } # [no_mangle] pub unsafe extern "C" fn LLVMInitializeARMTargetMC () { llvm_sys :: target :: LLVMInitializeARMTargetMC () } # [no_mangle] pub unsafe extern "C" fn LLVMInitializeARMAsmPrinter () { llvm_sys :: target :: LLVMInitializeARMAsmPrinter () } # [no_mangle] pub unsafe extern "C" fn LLVMInitializeARMAsmParser () { llvm_sys :: target :: LLVMInitializeARMAsmParser () } # [no_mangle] pub unsafe extern "C" fn LLVMInitializeARMDisassembler () { llvm_sys :: target :: LLVMInitializeARMDisassembler () } # [no_mangle] pub unsafe extern "C" fn LLVMInitializePowerPCTargetInfo () { llvm_sys :: target :: LLVMInitializePowerPCTargetInfo () } # [no_mangle] pub unsafe extern "C" fn LLVMInitializePowerPCTarget () { llvm_sys :: target :: LLVMInitializePowerPCTarget () } # [no_mangle] pub unsafe extern "C" fn LLVMInitializePowerPCTargetMC () { llvm_sys :: target :: LLVMInitializePowerPCTargetMC () } # [no_mangle] pub unsafe extern "C" fn LLVMInitializePowerPCAsmPrinter () { llvm_sys :: target :: LLVMInitializePowerPCAsmPrinter () } # [no_mangle] pub unsafe extern "C" fn LLVMInitializePowerPCAsmParser () { llvm_sys :: target :: LLVMInitializePowerPCAsmParser () } # [no_mangle] pub unsafe extern "C" fn LLVMInitializePowerPCDisassembler () { llvm_sys :: target :: LLVMInitializePowerPCDisassembler () } # [no_mangle] pub unsafe extern "C" fn LLVMInitializeSparcTargetInfo () { llvm_sys :: target :: LLVMInitializeSparcTargetInfo () } # [no_mangle] pub unsafe extern "C" fn LLVMInitializeSparcTarget () { llvm_sys :: target :: LLVMInitializeSparcTarget () } # [no_mangle] pub unsafe extern "C" fn LLVMInitializeSparcTargetMC () { llvm_sys :: target :: LLVMInitializeSparcTargetMC () } # [no_mangle] pub unsafe extern "C" fn LLVMInitializeSparcAsmPrinter () { llvm_sys :: target :: LLVMInitializeSparcAsmPrinter () } # [no_mangle] pub unsafe extern "C" fn LLVMInitializeSparcAsmParser () { llvm_sys :: target :: LLVMInitializeSparcAsmParser () } # [no_mangle] pub unsafe extern "C" fn LLVMInitializeSparcDisassembler () { llvm_sys :: target :: LLVMInitializeSparcDisassembler () } # [no_mangle] pub unsafe extern "C" fn LLVMInitializeX86TargetInfo () { llvm_sys :: target :: LLVMInitializeX86TargetInfo () } # [no_mangle] pub unsafe extern "C" fn LLVMInitializeX86Target () { llvm_sys :: target :: LLVMInitializeX86Target () } # [no_mangle] pub unsafe extern "C" fn LLVMInitializeX86TargetMC () { llvm_sys :: target :: LLVMInitializeX86TargetMC () } # [no_mangle] pub unsafe extern "C" fn LLVMInitializeX86AsmPrinter () { llvm_sys :: target :: LLVMInitializeX86AsmPrinter () } # [no_mangle] pub unsafe extern "C" fn LLVMInitializeX86AsmParser () { llvm_sys :: target :: LLVMInitializeX86AsmParser () } # [no_mangle] pub unsafe extern "C" fn LLVMInitializeX86Disassembler () { llvm_sys :: target :: LLVMInitializeX86Disassembler () } # [no_mangle] pub unsafe extern "C" fn LLVMInitializeBPFTargetInfo () { llvm_sys :: target :: LLVMInitializeBPFTargetInfo () } # [no_mangle] pub unsafe extern "C" fn LLVMInitializeBPFTarget () { llvm_sys :: target :: LLVMInitializeBPFTarget () } # [no_mangle] pub unsafe extern "C" fn LLVMInitializeBPFTargetMC () { llvm_sys :: target :: LLVMInitializeBPFTargetMC () } # [no_mangle] pub unsafe extern "C" fn LLVMInitializeBPFAsmPrinter () { llvm_sys :: target :: LLVMInitializeBPFAsmPrinter () } # [no_mangle] pub unsafe extern "C" fn LLVMInitializeBPFDisassembler () { llvm_sys :: target :: LLVMInitializeBPFDisassembler () } # [no_mangle] pub unsafe extern "C" fn LLVMInitializeLanaiTargetInfo () { llvm_sys :: target :: LLVMInitializeLanaiTargetInfo () } # [no_mangle] pub unsafe extern "C" fn LLVMInitializeLanaiTarget () { llvm_sys :: target :: LLVMInitializeLanaiTarget () } # [no_mangle] pub unsafe extern "C" fn LLVMInitializeLanaiTargetMC () { llvm_sys :: target :: LLVMInitializeLanaiTargetMC () } # [no_mangle] pub unsafe extern "C" fn LLVMInitializeLanaiAsmPrinter () { llvm_sys :: target :: LLVMInitializeLanaiAsmPrinter () } # [no_mangle] pub unsafe extern "C" fn LLVMInitializeLanaiAsmParser () { llvm_sys :: target :: LLVMInitializeLanaiAsmParser () } # [no_mangle] pub unsafe extern "C" fn LLVMInitializeLanaiDisassembler () { llvm_sys :: target :: LLVMInitializeLanaiDisassembler () } # [no_mangle] pub unsafe extern "C" fn LLVMInitializeRISCVTargetInfo () { llvm_sys :: target :: LLVMInitializeRISCVTargetInfo () } # [no_mangle] pub unsafe extern "C" fn LLVMInitializeRISCVTarget () { llvm_sys :: target :: LLVMInitializeRISCVTarget () } # [no_mangle] pub unsafe extern "C" fn LLVMInitializeRISCVTargetMC () { llvm_sys :: target :: LLVMInitializeRISCVTargetMC () } # [no_mangle] pub unsafe extern "C" fn LLVMInitializeRISCVAsmPrinter () { llvm_sys :: target :: LLVMInitializeRISCVAsmPrinter () } # [no_mangle] pub unsafe extern "C" fn LLVMInitializeRISCVAsmParser () { llvm_sys :: target :: LLVMInitializeRISCVAsmParser () } # [no_mangle] pub unsafe extern "C" fn LLVMInitializeRISCVDisassembler () { llvm_sys :: target :: LLVMInitializeRISCVDisassembler () } # [no_mangle] pub unsafe extern "C" fn LLVMInitializeWebAssemblyTargetInfo () { llvm_sys :: target :: LLVMInitializeWebAssemblyTargetInfo () } # [no_mangle] pub unsafe extern "C" fn LLVMInitializeWebAssemblyTarget () { llvm_sys :: target :: LLVMInitializeWebAssemblyTarget () } # [no_mangle] pub unsafe extern "C" fn LLVMInitializeWebAssemblyTargetMC () { llvm_sys :: target :: LLVMInitializeWebAssemblyTargetMC () } # [no_mangle] pub unsafe extern "C" fn LLVMInitializeWebAssemblyAsmPrinter () { llvm_sys :: target :: LLVMInitializeWebAssemblyAsmPrinter () } # [no_mangle] pub unsafe extern "C" fn LLVMInitializeWebAssemblyAsmParser () { llvm_sys :: target :: LLVMInitializeWebAssemblyAsmParser () } # [no_mangle] pub unsafe extern "C" fn LLVMInitializeWebAssemblyDisassembler () { llvm_sys :: target :: LLVMInitializeWebAssemblyDisassembler () } # [doc = " Get the data layout for a module."] # [no_mangle] pub unsafe extern "C" fn LLVMGetModuleDataLayout (M : LLVMModuleRef) -> LLVMTargetDataRef { llvm_sys :: target :: LLVMGetModuleDataLayout (M) } # [doc = " Set the data layout for a module."] # [no_mangle] pub unsafe extern "C" fn LLVMSetModuleDataLayout (M : LLVMModuleRef , R : LLVMTargetDataRef) { llvm_sys :: target :: LLVMSetModuleDataLayout (M , R) } # [doc = " Create target data from a target layout string."] # [no_mangle] pub unsafe extern "C" fn LLVMCreateTargetData (StringRep : * const :: libc :: c_char ,) -> LLVMTargetDataRef { llvm_sys :: target :: LLVMCreateTargetData (StringRep) } # [no_mangle] pub unsafe extern "C" fn LLVMAddTargetLibraryInfo (TLI : LLVMTargetLibraryInfoRef , PM : LLVMPassManagerRef ,) { llvm_sys :: target :: LLVMAddTargetLibraryInfo (TLI , PM) } # [no_mangle] pub unsafe extern "C" fn LLVMCopyStringRepOfTargetData (TD : LLVMTargetDataRef ,) -> * mut :: libc :: c_char { llvm_sys :: target :: LLVMCopyStringRepOfTargetData (TD) } # [no_mangle] pub unsafe extern "C" fn LLVMByteOrder (TD : LLVMTargetDataRef) -> LLVMByteOrdering { llvm_sys :: target :: LLVMByteOrder (TD) } # [no_mangle] pub unsafe extern "C" fn LLVMPointerSize (TD : LLVMTargetDataRef) -> :: libc :: c_uint { llvm_sys :: target :: LLVMPointerSize (TD) } # [no_mangle] pub unsafe extern "C" fn LLVMPointerSizeForAS (TD : LLVMTargetDataRef , AS : :: libc :: c_uint ,) -> :: libc :: c_uint { llvm_sys :: target :: LLVMPointerSizeForAS (TD , AS) } # [no_mangle] pub unsafe extern "C" fn LLVMIntPtrType (TD : LLVMTargetDataRef) -> LLVMTypeRef { llvm_sys :: target :: LLVMIntPtrType (TD) } # [no_mangle] pub unsafe extern "C" fn LLVMIntPtrTypeForAS (TD : LLVMTargetDataRef , AS : :: libc :: c_uint ,) -> LLVMTypeRef { llvm_sys :: target :: LLVMIntPtrTypeForAS (TD , AS) } # [no_mangle] pub unsafe extern "C" fn LLVMIntPtrTypeInContext (C : LLVMContextRef , TD : LLVMTargetDataRef ,) -> LLVMTypeRef { llvm_sys :: target :: LLVMIntPtrTypeInContext (C , TD) } # [no_mangle] pub unsafe extern "C" fn LLVMIntPtrTypeForASInContext (C : LLVMContextRef , TD : LLVMTargetDataRef , AS : :: libc :: c_uint ,) -> LLVMTypeRef { llvm_sys :: target :: LLVMIntPtrTypeForASInContext (C , TD , AS) } # [no_mangle] pub unsafe extern "C" fn LLVMSizeOfTypeInBits (TD : LLVMTargetDataRef , Ty : LLVMTypeRef ,) -> :: libc :: c_ulonglong { llvm_sys :: target :: LLVMSizeOfTypeInBits (TD , Ty) } # [no_mangle] pub unsafe extern "C" fn LLVMStoreSizeOfType (TD : LLVMTargetDataRef , Ty : LLVMTypeRef ,) -> :: libc :: c_ulonglong { llvm_sys :: target :: LLVMStoreSizeOfType (TD , Ty) } # [no_mangle] pub unsafe extern "C" fn LLVMABISizeOfType (TD : LLVMTargetDataRef , Ty : LLVMTypeRef ,) -> :: libc :: c_ulonglong { llvm_sys :: target :: LLVMABISizeOfType (TD , Ty) } # [no_mangle] pub unsafe extern "C" fn LLVMABIAlignmentOfType (TD : LLVMTargetDataRef , Ty : LLVMTypeRef ,) -> :: libc :: c_uint { llvm_sys :: target :: LLVMABIAlignmentOfType (TD , Ty) } # [no_mangle] pub unsafe extern "C" fn LLVMCallFrameAlignmentOfType (TD : LLVMTargetDataRef , Ty : LLVMTypeRef ,) -> :: libc :: c_uint { llvm_sys :: target :: LLVMCallFrameAlignmentOfType (TD , Ty) } # [no_mangle] pub unsafe extern "C" fn LLVMPreferredAlignmentOfType (TD : LLVMTargetDataRef , Ty : LLVMTypeRef ,) -> :: libc :: c_uint { llvm_sys :: target :: LLVMPreferredAlignmentOfType (TD , Ty) } # [no_mangle] pub unsafe extern "C" fn LLVMPreferredAlignmentOfGlobal (TD : LLVMTargetDataRef , GlobalVar : LLVMValueRef ,) -> :: libc :: c_uint { llvm_sys :: target :: LLVMPreferredAlignmentOfGlobal (TD , GlobalVar) } # [no_mangle] pub unsafe extern "C" fn LLVMElementAtOffset (TD : LLVMTargetDataRef , StructTy : LLVMTypeRef , Offset : :: libc :: c_ulonglong ,) -> :: libc :: c_uint { llvm_sys :: target :: LLVMElementAtOffset (TD , StructTy , Offset) } # [no_mangle] pub unsafe extern "C" fn LLVMOffsetOfElement (TD : LLVMTargetDataRef , StructTy : LLVMTypeRef , Element : :: libc :: c_uint ,) -> :: libc :: c_ulonglong { llvm_sys :: target :: LLVMOffsetOfElement (TD , StructTy , Element) } # [no_mangle] pub unsafe extern "C" fn LLVMDisposeTargetData (TD : LLVMTargetDataRef) { llvm_sys :: target :: LLVMDisposeTargetData (TD) } # [no_mangle] pub unsafe extern "C" fn LLVM_InitializeAllTargetInfos () { llvm_sys :: target :: LLVM_InitializeAllTargetInfos () } # [no_mangle] pub unsafe extern "C" fn LLVM_InitializeAllTargets () { llvm_sys :: target :: LLVM_InitializeAllTargets () } # [no_mangle] pub unsafe extern "C" fn LLVM_InitializeAllTargetMCs () { llvm_sys :: target :: LLVM_InitializeAllTargetMCs () } # [no_mangle] pub unsafe extern "C" fn LLVM_InitializeAllAsmPrinters () { llvm_sys :: target :: LLVM_InitializeAllAsmPrinters () } # [no_mangle] pub unsafe extern "C" fn LLVM_InitializeAllAsmParsers () { llvm_sys :: target :: LLVM_InitializeAllAsmParsers () } # [no_mangle] pub unsafe extern "C" fn LLVM_InitializeAllDisassemblers () { llvm_sys :: target :: LLVM_InitializeAllDisassemblers () } # [doc = " Returns 1 on failure."] # [no_mangle] pub unsafe extern "C" fn LLVM_InitializeNativeTarget () -> LLVMBool { llvm_sys :: target :: LLVM_InitializeNativeTarget () } # [doc = " Returns 1 on failure."] # [no_mangle] pub unsafe extern "C" fn LLVM_InitializeNativeAsmParser () -> LLVMBool { llvm_sys :: target :: LLVM_InitializeNativeAsmParser () } # [doc = " Returns 1 on failure."] # [no_mangle] pub unsafe extern "C" fn LLVM_InitializeNativeAsmPrinter () -> LLVMBool { llvm_sys :: target :: LLVM_InitializeNativeAsmPrinter () } # [doc = " Returns 1 on failure."] # [no_mangle] pub unsafe extern "C" fn LLVM_InitializeNativeDisassembler () -> LLVMBool { llvm_sys :: target :: LLVM_InitializeNativeDisassembler () }