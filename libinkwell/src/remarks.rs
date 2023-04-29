# ! [doc = " Remark diagnostics library."] use llvm_sys :: prelude :: LLVMBool ; pub use llvm_sys :: remarks :: * ; # [doc = " Returns the buffer holding the string."] # [no_mangle] pub unsafe extern "C" fn LLVMRemarkStringGetData (String : LLVMRemarkStringRef ,) -> * const :: libc :: c_char { llvm_sys :: remarks :: LLVMRemarkStringGetData (String) } # [doc = " Returns the size of the string."] # [no_mangle] pub unsafe extern "C" fn LLVMRemarkStringGetLen (String : LLVMRemarkStringRef) -> u32 { llvm_sys :: remarks :: LLVMRemarkStringGetLen (String) } # [doc = " Return the path to the source file for a debug location."] # [no_mangle] pub unsafe extern "C" fn LLVMRemarkDebugLocGetSourceFilePath (DL : LLVMRemarkDebugLocRef ,) -> LLVMRemarkStringRef { llvm_sys :: remarks :: LLVMRemarkDebugLocGetSourceFilePath (DL) } # [doc = " Return the line in the source file for a debug location."] # [no_mangle] pub unsafe extern "C" fn LLVMRemarkDebugLocGetSourceLine (DL : LLVMRemarkDebugLocRef) -> u32 { llvm_sys :: remarks :: LLVMRemarkDebugLocGetSourceLine (DL) } # [doc = " Return the column in the source file for a debug location."] # [no_mangle] pub unsafe extern "C" fn LLVMRemarkDebugLocGetSourceColumn (DL : LLVMRemarkDebugLocRef) -> u32 { llvm_sys :: remarks :: LLVMRemarkDebugLocGetSourceColumn (DL) } # [doc = " Returns the key of an argument. The key defines what the value is, and the"] # [doc = " same key can appear multiple times in the list of arguments."] # [no_mangle] pub unsafe extern "C" fn LLVMRemarkArgGetKey (Arg : LLVMRemarkArgRef) -> LLVMRemarkStringRef { llvm_sys :: remarks :: LLVMRemarkArgGetKey (Arg) } # [doc = " Returns the value of an argument. This is a string that can contain newlines."] # [no_mangle] pub unsafe extern "C" fn LLVMRemarkArgGetValue (Arg : LLVMRemarkArgRef) -> LLVMRemarkStringRef { llvm_sys :: remarks :: LLVMRemarkArgGetValue (Arg) } # [doc = " Returns the debug location that is attached to the value of this argument."] # [no_mangle] pub unsafe extern "C" fn LLVMRemarkArgGetDebugLoc (Arg : LLVMRemarkArgRef) -> LLVMRemarkDebugLocRef { llvm_sys :: remarks :: LLVMRemarkArgGetDebugLoc (Arg) } # [doc = " Free the resources used by the remark entry."] # [no_mangle] pub unsafe extern "C" fn LLVMRemarkEntryDispose (Remark : LLVMRemarkEntryRef) { llvm_sys :: remarks :: LLVMRemarkEntryDispose (Remark) } # [doc = " The type of the remark. For example, it can allow users to only keep the"] # [doc = " missed optimizations from the compiler."] # [no_mangle] pub unsafe extern "C" fn LLVMRemarkEntryGetType (Remark : LLVMRemarkEntryRef) -> LLVMRemarkType { llvm_sys :: remarks :: LLVMRemarkEntryGetType (Remark) } # [doc = " Get the name of the pass that emitted this remark."] # [no_mangle] pub unsafe extern "C" fn LLVMRemarkEntryGetPassName (Remark : LLVMRemarkEntryRef ,) -> LLVMRemarkStringRef { llvm_sys :: remarks :: LLVMRemarkEntryGetPassName (Remark) } # [doc = " Get an identifier of the remark."] # [no_mangle] pub unsafe extern "C" fn LLVMRemarkEntryGetRemarkName (Remark : LLVMRemarkEntryRef ,) -> LLVMRemarkStringRef { llvm_sys :: remarks :: LLVMRemarkEntryGetRemarkName (Remark) } # [doc = " Get the name of the function being processed when the remark was emitted."] # [no_mangle] pub unsafe extern "C" fn LLVMRemarkEntryGetFunctionName (Remark : LLVMRemarkEntryRef ,) -> LLVMRemarkStringRef { llvm_sys :: remarks :: LLVMRemarkEntryGetFunctionName (Remark) } # [doc = " Returns the debug location that is attached to this remark."] # [no_mangle] pub unsafe extern "C" fn LLVMRemarkEntryGetDebugLoc (Remark : LLVMRemarkEntryRef ,) -> LLVMRemarkDebugLocRef { llvm_sys :: remarks :: LLVMRemarkEntryGetDebugLoc (Remark) } # [doc = " Return the hotness of the remark."] # [no_mangle] pub unsafe extern "C" fn LLVMRemarkEntryGetHotness (Remark : LLVMRemarkEntryRef) -> u64 { llvm_sys :: remarks :: LLVMRemarkEntryGetHotness (Remark) } # [doc = " The number of arguments the remark holds."] # [no_mangle] pub unsafe extern "C" fn LLVMRemarkEntryGetNumArgs (Remark : LLVMRemarkEntryRef) -> u32 { llvm_sys :: remarks :: LLVMRemarkEntryGetNumArgs (Remark) } # [doc = " Get a new iterator to iterate over a remark's argument."] # [no_mangle] pub unsafe extern "C" fn LLVMRemarkEntryGetFirstArg (Remark : LLVMRemarkEntryRef ,) -> LLVMRemarkArgRef { llvm_sys :: remarks :: LLVMRemarkEntryGetFirstArg (Remark) } # [doc = " Get the next argument in Remark from the position of It."] # [no_mangle] pub unsafe extern "C" fn LLVMRemarkEntryGetNextArg (It : LLVMRemarkArgRef , Remark : LLVMRemarkEntryRef ,) -> LLVMRemarkArgRef { llvm_sys :: remarks :: LLVMRemarkEntryGetNextArg (It , Remark) } # [doc = " Creates a remark parser that can be used to parse the buffer located in"] # [doc = " Buf of size Size bytes."] # [no_mangle] pub unsafe extern "C" fn LLVMRemarkParserCreateYAML (Buf : * const :: libc :: c_void , Size : u64 ,) -> LLVMRemarkParserRef { llvm_sys :: remarks :: LLVMRemarkParserCreateYAML (Buf , Size) } # [no_mangle] pub unsafe extern "C" fn LLVMRemarkParserCreateBitstream (Buf : * const :: libc :: c_void , Size : u64 ,) -> LLVMRemarkParserRef { llvm_sys :: remarks :: LLVMRemarkParserCreateBitstream (Buf , Size) } # [doc = " Returns the next remark in the file."] # [no_mangle] pub unsafe extern "C" fn LLVMRemarkParserGetNext (Parser : LLVMRemarkParserRef ,) -> LLVMRemarkEntryRef { llvm_sys :: remarks :: LLVMRemarkParserGetNext (Parser) } # [doc = " Returns `1` if the parser encountered an error while parsing the buffer."] # [no_mangle] pub unsafe extern "C" fn LLVMRemarkParserHasError (Parser : LLVMRemarkParserRef) -> LLVMBool { llvm_sys :: remarks :: LLVMRemarkParserHasError (Parser) } # [doc = " Returns a null-terminated string containing an error message."] # [no_mangle] pub unsafe extern "C" fn LLVMRemarkParserGetErrorMessage (Parser : LLVMRemarkParserRef ,) -> * const :: libc :: c_char { llvm_sys :: remarks :: LLVMRemarkParserGetErrorMessage (Parser) } # [no_mangle] pub unsafe extern "C" fn LLVMRemarkParserDispose (Parser : LLVMRemarkParserRef) { llvm_sys :: remarks :: LLVMRemarkParserDispose (Parser) } # [doc = " Returns the version of the remarks library."] # [no_mangle] pub unsafe extern "C" fn LLVMRemarkVersion () -> u32 { llvm_sys :: remarks :: LLVMRemarkVersion () }