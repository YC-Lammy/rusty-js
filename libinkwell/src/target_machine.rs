# ! [doc = " Target machine information, to generate assembly or object files."] use llvm_sys :: prelude :: * ; use llvm_sys :: target :: LLVMTargetDataRef ; pub use llvm_sys :: target_machine :: * ; # [no_mangle] pub unsafe extern "C" fn LLVMGetFirstTarget () -> LLVMTargetRef { llvm_sys :: target_machine :: LLVMGetFirstTarget () } # [no_mangle] pub unsafe extern "C" fn LLVMGetNextTarget (T : LLVMTargetRef) -> LLVMTargetRef { llvm_sys :: target_machine :: LLVMGetNextTarget (T) } # [no_mangle] pub unsafe extern "C" fn LLVMGetTargetFromName (Name : * const :: libc :: c_char) -> LLVMTargetRef { llvm_sys :: target_machine :: LLVMGetTargetFromName (Name) } # [no_mangle] pub unsafe extern "C" fn LLVMGetTargetFromTriple (Triple : * const :: libc :: c_char , T : * mut LLVMTargetRef , ErrorMessage : * mut * mut :: libc :: c_char ,) -> LLVMBool { llvm_sys :: target_machine :: LLVMGetTargetFromTriple (Triple , T , ErrorMessage) } # [no_mangle] pub unsafe extern "C" fn LLVMGetTargetName (T : LLVMTargetRef) -> * const :: libc :: c_char { llvm_sys :: target_machine :: LLVMGetTargetName (T) } # [no_mangle] pub unsafe extern "C" fn LLVMGetTargetDescription (T : LLVMTargetRef) -> * const :: libc :: c_char { llvm_sys :: target_machine :: LLVMGetTargetDescription (T) } # [no_mangle] pub unsafe extern "C" fn LLVMTargetHasJIT (T : LLVMTargetRef) -> LLVMBool { llvm_sys :: target_machine :: LLVMTargetHasJIT (T) } # [no_mangle] pub unsafe extern "C" fn LLVMTargetHasTargetMachine (T : LLVMTargetRef) -> LLVMBool { llvm_sys :: target_machine :: LLVMTargetHasTargetMachine (T) } # [no_mangle] pub unsafe extern "C" fn LLVMTargetHasAsmBackend (T : LLVMTargetRef) -> LLVMBool { llvm_sys :: target_machine :: LLVMTargetHasAsmBackend (T) } # [no_mangle] pub unsafe extern "C" fn LLVMCreateTargetMachine (T : LLVMTargetRef , Triple : * const :: libc :: c_char , CPU : * const :: libc :: c_char , Features : * const :: libc :: c_char , Level : LLVMCodeGenOptLevel , Reloc : LLVMRelocMode , CodeModel : LLVMCodeModel ,) -> LLVMTargetMachineRef { llvm_sys :: target_machine :: LLVMCreateTargetMachine (T , Triple , CPU , Features , Level , Reloc , CodeModel) } # [no_mangle] pub unsafe extern "C" fn LLVMDisposeTargetMachine (T : LLVMTargetMachineRef) { llvm_sys :: target_machine :: LLVMDisposeTargetMachine (T) } # [no_mangle] pub unsafe extern "C" fn LLVMGetTargetMachineTarget (T : LLVMTargetMachineRef) -> LLVMTargetRef { llvm_sys :: target_machine :: LLVMGetTargetMachineTarget (T) } # [no_mangle] pub unsafe extern "C" fn LLVMGetTargetMachineTriple (T : LLVMTargetMachineRef ,) -> * mut :: libc :: c_char { llvm_sys :: target_machine :: LLVMGetTargetMachineTriple (T) } # [no_mangle] pub unsafe extern "C" fn LLVMGetTargetMachineCPU (T : LLVMTargetMachineRef) -> * mut :: libc :: c_char { llvm_sys :: target_machine :: LLVMGetTargetMachineCPU (T) } # [no_mangle] pub unsafe extern "C" fn LLVMGetTargetMachineFeatureString (T : LLVMTargetMachineRef ,) -> * mut :: libc :: c_char { llvm_sys :: target_machine :: LLVMGetTargetMachineFeatureString (T) } # [doc = " Create a DataLayout based on the target machine."] # [no_mangle] pub unsafe extern "C" fn LLVMCreateTargetDataLayout (T : LLVMTargetMachineRef) -> LLVMTargetDataRef { llvm_sys :: target_machine :: LLVMCreateTargetDataLayout (T) } # [no_mangle] pub unsafe extern "C" fn LLVMSetTargetMachineAsmVerbosity (T : LLVMTargetMachineRef , VerboseAsm : LLVMBool ,) { llvm_sys :: target_machine :: LLVMSetTargetMachineAsmVerbosity (T , VerboseAsm) } # [no_mangle] pub unsafe extern "C" fn LLVMTargetMachineEmitToFile (T : LLVMTargetMachineRef , M : LLVMModuleRef , Filename : * mut :: libc :: c_char , codegen : LLVMCodeGenFileType , ErrorMessage : * mut * mut :: libc :: c_char ,) -> LLVMBool { llvm_sys :: target_machine :: LLVMTargetMachineEmitToFile (T , M , Filename , codegen , ErrorMessage) } # [no_mangle] pub unsafe extern "C" fn LLVMTargetMachineEmitToMemoryBuffer (T : LLVMTargetMachineRef , M : LLVMModuleRef , codegen : LLVMCodeGenFileType , ErrorMessage : * mut * mut :: libc :: c_char , OutMemBuf : * mut LLVMMemoryBufferRef ,) -> LLVMBool { llvm_sys :: target_machine :: LLVMTargetMachineEmitToMemoryBuffer (T , M , codegen , ErrorMessage , OutMemBuf) } # [no_mangle] pub unsafe extern "C" fn LLVMGetDefaultTargetTriple () -> * mut :: libc :: c_char { llvm_sys :: target_machine :: LLVMGetDefaultTargetTriple () } # [doc = " Normalize a target triple. The result needs to be disposed with LLVMDisposeMessage."] # [no_mangle] pub unsafe extern "C" fn LLVMNormalizeTargetTriple (triple : * const :: libc :: c_char ,) -> * mut :: libc :: c_char { llvm_sys :: target_machine :: LLVMNormalizeTargetTriple (triple) } # [doc = " Get the host CPU as a string. The result needs to be disposed with LLVMDisposeMessage."] # [no_mangle] pub unsafe extern "C" fn LLVMGetHostCPUName () -> * mut :: libc :: c_char { llvm_sys :: target_machine :: LLVMGetHostCPUName () } # [doc = " Get the host CPU's features as a string. The result needs to be disposed with LLVMDisposeMessage."] # [no_mangle] pub unsafe extern "C" fn LLVMGetHostCPUFeatures () -> * mut :: libc :: c_char { llvm_sys :: target_machine :: LLVMGetHostCPUFeatures () } # [no_mangle] pub unsafe extern "C" fn LLVMAddAnalysisPasses (T : LLVMTargetMachineRef , PM : LLVMPassManagerRef) { llvm_sys :: target_machine :: LLVMAddAnalysisPasses (T , PM) }