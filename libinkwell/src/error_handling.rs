pub use llvm_sys :: error_handling :: LLVMFatalErrorHandler ; # [doc = " Install a fatal error handler."] # [doc = ""] # [doc = " LLVM will call `exit(1)` if it detects a fatal error. A callback"] # [doc = " registered with this function will be invoked before the program is"] # [doc = " exited."] # [no_mangle] pub unsafe extern "C" fn LLVMInstallFatalErrorHandler (Handler : LLVMFatalErrorHandler) { llvm_sys :: error_handling :: LLVMInstallFatalErrorHandler (Handler) } # [doc = " Reset fatal error handling to the default."] # [no_mangle] pub unsafe extern "C" fn LLVMResetFatalErrorHandler () { llvm_sys :: error_handling :: LLVMResetFatalErrorHandler () } # [doc = " Enable LLVM's build-in stack trace code."] # [doc = ""] # [doc = " This intercepts the OS's crash signals and prints which component"] # [doc = " of LLVM you were in at the time of the crash."] # [no_mangle] pub unsafe extern "C" fn LLVMEnablePrettyStackTrace () { llvm_sys :: error_handling :: LLVMEnablePrettyStackTrace () }