# ! [doc = " The IR reader"] use llvm_sys :: prelude :: * ; # [doc = " Read LLVM IR from a memory buffer and convert it to an in-memory Module."] # [doc = ""] # [doc = " Returns 0 on success, and an optional human-readable description of any"] # [doc = " errors that occurred."] # [no_mangle] pub unsafe extern "C" fn LLVMParseIRInContext (ContextRef : LLVMContextRef , MemBuf : LLVMMemoryBufferRef , OutM : * mut LLVMModuleRef , OutMessage : * mut * mut :: libc :: c_char ,) -> LLVMBool { llvm_sys :: ir_reader :: LLVMParseIRInContext (ContextRef , MemBuf , OutM , OutMessage) }