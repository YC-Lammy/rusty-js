use llvm_sys :: prelude :: * ; # [no_mangle] pub unsafe extern "C" fn LLVMLoadLibraryPermanently (Filename : * const :: libc :: c_char) -> LLVMBool { llvm_sys :: support :: LLVMLoadLibraryPermanently (Filename) } # [no_mangle] pub unsafe extern "C" fn LLVMParseCommandLineOptions (argc : :: libc :: c_int , argv : * const * const :: libc :: c_char , Overview : * const :: libc :: c_char ,) { llvm_sys :: support :: LLVMParseCommandLineOptions (argc , argv , Overview) } # [doc = " Search all previously loaded dynamic libraries for the named symbol."] # [doc = ""] # [doc = " Returns its address if found, otherwise null."] # [doc = ""] # [doc = " Added in LLVM 3.7."] # [no_mangle] pub unsafe extern "C" fn LLVMSearchForAddressOfSymbol (symbolName : * const :: libc :: c_char ,) -> * mut :: libc :: c_void { llvm_sys :: support :: LLVMSearchForAddressOfSymbol (symbolName) } # [doc = " Permanently add the named symbol with the provided value."] # [doc = ""] # [doc = " Symbols added this way are searched before any libraries."] # [doc = ""] # [doc = " Added in LLVM 3.7."] # [no_mangle] pub unsafe extern "C" fn LLVMAddSymbol (symbolName : * const :: libc :: c_char , symbolValue : * mut :: libc :: c_void ,) { llvm_sys :: support :: LLVMAddSymbol (symbolName , symbolValue) }