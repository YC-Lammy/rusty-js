use crate::runtime::{ClassID, ConstID, FuncID, RegexID, StringID, TemplateID};
use crate::types::JValue;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Hash)]
pub struct Register(pub u8);

impl std::fmt::Display for Register {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("#r{}", self.0))
    }
}

impl std::fmt::Debug for Register {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("#r{}", self.0))
    }
}

#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub struct Block(pub u16);

impl std::fmt::Debug for Block {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("@block({})", self.0))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum LoopHint {
    While,
    For,
    ForOf,
    ForIn,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeclareKind {
    Var,
    Let,
    Const,
    None,
}

/// a value in the TempAlloc array
#[repr(packed)]
pub struct TempAllocValue {
    /// an opaque data
    pub flag: u8,
    /// the jsvalue
    pub value: JValue,
}

/// an encoded value representing two registers
/// 
/// this Enum is used to fit the opcode's size
#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum CompactRegister {
    T0R0,
    T0R1,
    T0R2,
    T0R3,

    T1R0,
    T1R1,
    T1R2,
    T1R3,

    T2R0,
    T2R1,
    T2R2,
    T2R3,

    T3R0,
    T3R1,
    T3R2,
    T3R3,
}

impl CompactRegister {
    /// the first register of this enum
    pub fn target(&self) -> Register {
        match self {
            Self::T0R0 | Self::T0R1 | Self::T0R2 | Self::T0R3 => Register(0),
            Self::T1R0 | Self::T1R1 | Self::T1R2 | Self::T1R3 => Register(1),
            Self::T2R0 | Self::T2R1 | Self::T2R2 | Self::T2R3 => Register(2),
            Self::T3R0 | Self::T3R1 | Self::T3R2 | Self::T3R3 => Register(3),
        }
    }

    /// the second register of this enum
    pub fn value(&self) -> Register {
        match self {
            Self::T0R0 | Self::T1R0 | Self::T2R0 | Self::T3R0 => Register(0),
            Self::T0R1 | Self::T1R1 | Self::T2R1 | Self::T3R1 => Register(1),
            Self::T0R2 | Self::T1R2 | Self::T2R2 | Self::T3R2 => Register(2),
            Self::T0R3 | Self::T1R3 | Self::T2R3 | Self::T3R3 => Register(3),
        }
    }
}

/// convert two registers into a compact one
impl From<(Register, Register)> for CompactRegister {
    fn from(rs: (Register, Register)) -> Self {
        match rs.0 .0 {
            0 => match rs.1 .0 {
                0 => Self::T0R0,
                1 => Self::T0R1,
                2 => Self::T0R2,
                3 => Self::T0R3,
                _ => unreachable!(),
            },
            1 => match rs.1 .0 {
                0 => Self::T1R0,
                1 => Self::T1R1,
                2 => Self::T1R2,
                3 => Self::T1R3,
                _ => unreachable!(),
            },
            2 => match rs.1 .0 {
                0 => Self::T2R0,
                1 => Self::T2R1,
                2 => Self::T2R2,
                3 => Self::T2R3,
                _ => unreachable!(),
            },
            3 => match rs.1 .0 {
                0 => Self::T3R0,
                1 => Self::T3R1,
                2 => Self::T3R2,
                3 => Self::T3R3,
                _ => unreachable!(),
            },
            _ => unreachable!(),
        }
    }
}

/// stackoffset is not offset in bytes, it is offset in JValues,
/// 
/// to get the offset in bytes: offset * size_of JValue .
/// 
/// OpCodes are 64bit width
#[derive(Debug, Clone, Copy)]
pub enum OpCode {
    /// no op
    NoOp,
    /// call the debugger, if the debugger on runtime is not set, 
    /// this is a no op
    Debugger,

    /// mov from register to another,
    /// if the two registers are the same, this is a no op
    Mov {
        from: Register,
        to: Register,
    },

    /// return b if a is undefined, else a
    Select {
        a: Register,
        b: Register,
        result: Register,
    },
    /// return a if t is true, else b
    CondSelect {
        t: Register,
        a: Register,
        b: Register,
        result: Register,
    },

    /// create a code block, but not switch to it
    CreateBlock(Block),
    /// switch to a code block, operations after this point will be written into this block.
    SwitchToBlock(Block),

    /// jump to a code block
    Jump {
        to: Block,
        /// reserve for future purpose
        line:u32,
    },
    JumpIfTrue {
        value: Register,
        to: Block,
    },
    JumpIfFalse {
        value: Register,
        to: Block,
    },
    /// jump to a code block if the iterator in scope is done
    JumpIfIterDone {
        to: Block,
    },
    /// enters a loop, this op is only used by the interpreter
    Loop{
        body_start:u32,
        body_len:u16,
    },
    /// break without label, only used by the interpreter
    Break{
        exit:Block,
    },
    /// break if value is true, only used by the interpreter
    BreakIfTrue{
        value:Register,
        exit:Block,
    },
    /// break if value is false, only used by the interpreter
    BreakIfFalse{
        value:Register,
        exit:Block,
    },
    /// break if iterator is done, only used by the interpreter
    BreakIfIterDone{
        exit:Block
    },

    /// enter a try context, when an error occours, will jump to the catch block
    EnterTry {
        catch_block: Block,
    },
    /// finish try
    ExitTry,

    /// throw and cause a panic
    Throw {
        value: Register,
    },

    /// return the value
    Return {
        value: Register,
    },

    /// create a list of arguments on stack
    CreateArg {
        stack_offset: u16,
        len: u32,
    },
    /// write an argument to its location on stack
    PushArg {
        value: Register,
        stack_offset: u16,
    },
    /// write an argument that needs to be spread
    PushArgSpread {
        value: Register,
        stack_offset: u16,
    },
    /// finish the creation of arguments,
    /// 
    /// if spread is required for any argument, spread it now
    FinishArgs {
        base_stack_offset: u16,
        len: u16,
    },

    /// allocate memory and store in alloc register,
    /// this op is supposed to store values temporary
    /// and is expected to deallocate immediately.
    TempAlloc {
        /// size in bytes
        size: u32,
    },
    /// dealoocate the allocated memory
    TempDealloc {
        /// size in bytes
        size: u32,
    },
    /// write a value into the allocated memory
    StoreTempAlloc {
        /// offset in bytes
        offset: u16,
        /// opaque data
        flag: u8,
        value: Register,
    },
    /// read a value from the allocated memory
    ReadTempAlloc {
        /// offset in bytes
        offset: u16,
        result: Register,
    },

    /// create a temporary slot
    StoreTemp {
        value: Register,
    },
    /// read from current temporary slot
    ReadTemp {
        value: Register,
    },
    /// release current temporary slot
    ReleaseTemp,

    /// declared a dynamic variable
    DeclareDynamicVar {
        from: Register,
        kind: DeclareKind,
        id: u32,
    },
    /// write to a dynamic variable,
    /// 
    /// if the variable is not declared, this will write to global object
    WriteDynamicVar {
        from: Register,
        id: u32,
    },
    /// read from a dynamic variable
    /// 
    /// if the variable is not declared, this will read from gloabl object
    ReadDynamicVar {
        result: Register,
        id: u32,
    },

    /// capture variable from stack. 
    /// 
    /// the bytecode builder will no longer use the stack offset as variable,
    /// read and write of the dedicated variable will not be done on stack,
    /// but on the capture environment.
    Capture {
        stack_offset: u16,
        capture_stack_offset: u16,
    },
    /// read a variable from the capture environment
    ReadCapturedVar {
        result: Register,
        offset: u16,
    },
    /// write a variable to the capture environment
    WriteCapturedVar {
        from: Register,
        offset: u16,
    },

    /// write to the stack
    WriteToStack {
        from: Register,
        stack_offset: u16,
    },
    /// read from the stack
    ReadFromStack {
        result: Register,
        stack_offset: u16,
    },
    /// read a param
    ReadParam {
        result: Register,
        index: u32,
    },
    /// collect all the params to an array starting from index.
    CollectParam {
        result: Register,
        /// the starting index
        start: u32,
    },

    /// call a function using the stack + offset.
    /// 
    /// arguments are already on the stack
    Call {
        result: Register,
        this: Register,
        callee: Register,
        stack_offset: u16,
    },
    /// call if callee is not null or undefined
    CallOptChain {
        result: Register,
        this: Register,
        callee: Register,
        stack_offset: u16,
    },
    /// ivoke a new operation
    New {
        result: Register,
        callee: Register,
        stack_offset: u16,
    },
    /// get the metadata new.target
    NewTarget {
        result: Register,
    },
    /// get the metadata import.meta
    ImportMeta {
        result: Register,
    },

    /// create an iterator from a value
    IntoIter {
        target: Register,
        /// the type of loop
        hint: LoopHint,
    },
    /// get the next value from the iterator
    IterNext {
        result: Register,
        hint: LoopHint,
        stack_offset: u16,
    },
    /// collect all the unread values from the iterator
    IterCollect {
        result: Register,
        stack_offset: u16,
    },
    /// destroy the current iterator and restore the preveous one
    IterDrop,

    Add {
        result: Register,
        left: Register,
        right: Register,
    },
    Sub {
        result: Register,
        left: Register,
        right: Register,
    },
    Mul {
        result: Register,
        left: Register,
        right: Register,
    },
    Div {
        result: Register,
        left: Register,
        right: Register,
    },
    Exp {
        result: Register,
        left: Register,
        right: Register,
    },
    LShift {
        result: Register,
        left: Register,
        right: Register,
    },
    RShift {
        result: Register,
        left: Register,
        right: Register,
    },
    ZeroFillRShift {
        result: Register,
        left: Register,
        right: Register,
    },
    Rem {
        result: Register,
        left: Register,
        right: Register,
    },

    Await {
        result: Register,
        future: Register,
    },
    Yield {
        result: Register,
        arg: Register,
    },

    And {
        result: Register,
        left: Register,
        right: Register,
    },
    Or {
        result: Register,
        left: Register,
        right: Register,
    },
    Not {
        result: Register,
        right: Register,
    },
    BitNot {
        result: Register,
        right: Register,
    },
    BitAnd {
        result: Register,
        left: Register,
        right: Register,
    },
    BitOr {
        result: Register,
        left: Register,
        right: Register,
    },
    BitXor {
        result: Register,
        left: Register,
        right: Register,
    },
    Minus {
        result: Register,
        right: Register,
    },
    Plus {
        result: Register,
        right: Register,
    },
    /// return null if not null
    Nullish {
        result: Register,
        left: Register,
        right: Register,
    },

    EqEq {
        result: Register,
        left: Register,
        right: Register,
    },
    EqEqEq {
        result: Register,
        left: Register,
        right: Register,
    },
    NotEq {
        result: Register,
        left: Register,
        right: Register,
    },
    NotEqEq {
        result: Register,
        left: Register,
        right: Register,
    },
    Gt {
        result: Register,
        left: Register,
        right: Register,
    },
    GtEq {
        result: Register,
        left: Register,
        right: Register,
    },
    Lt {
        result: Register,
        left: Register,
        right: Register,
    },
    LtEq {
        result: Register,
        left: Register,
        right: Register,
    },

    In {
        result: Register,
        left: Register,
        right: Register,
    },
    PrivateIn {
        result: Register,
        name: u32,
        right: Register,
    },
    InstanceOf {
        result: Register,
        left: Register,
        right: Register,
    },
    TypeOf {
        result: Register,
        right: Register,
    },

    /// write the value of field that is loaded into register
    WriteField {
        obj: Register,
        field: Register,
        value: Register,
        stack_offset: u16,
    },
    ReadField {
        obj: Register,
        field: Register,
        result: Register,
        stack_offset: u16,
    },
    /// read the value if obj is not null or undefined
    ReadFieldOptChain {
        obj: Register,
        field: Register,
        result: Register,
        stack_offset: u16,
    },
    /// write the value if obj is not null or undefined
    WriteFieldOptChain {
        obj: Register,
        field: Register,
        from: Register,
        stack_offset: u16,
    },
    /// write the field of an object with a static field name
    WriteFieldStatic {
        obj_value: CompactRegister,
        field_id: u32,
        stack_offset: u16,
    },
    /// read the field of an object with a static field name
    ReadFieldStatic {
        obj_result: CompactRegister,
        field_id: u32,
        stack_offset: u16,
    },
    /// read the value if obj is not null or undefined
    ReadFieldStaticOptChain {
        obj_result: CompactRegister,
        field_id: u32,
        stack_offset: u16,
    },
    /// remove a field from an object
    RemoveFieldStatic {
        obj: Register,
        field_id: u32,
    },

    /// bind a setter into an object
    BindSetter {
        obj: Register,
        field_id: u32,
        setter: Register,
    },
    /// bind a getter into an object
    BindGetter {
        obj: Register,
        field_id: u32,
        getter: Register,
    },
    /// extend an object by an object
    ExtendObject {
        obj: Register,
        from: Register,
    },
    /// read from field of the super object
    ReadSuperField {
        constructor_result: CompactRegister,
        field: Register,
        stack_offset: u16,
    },
    ReadSuperFieldStatic {
        constructor_result: CompactRegister,
        field_id: u32,
        stack_offset: u16,
    },
    WriteSuperField {
        constructor_value: CompactRegister,
        field: Register,
        stack_offset: u16,
    },
    WriteSuperFieldStatic {
        constructor_value: CompactRegister, // 8bit
        field: u32,                         // 32 bit
        stack_offset: u16,                  // 16bit
    },

    LoadStaticString {
        result: Register,
        id: StringID,
    },
    LoadStaticFloat32 {
        result: Register,
        value: f32,
    },
    LoadStaticFloat {
        result: Register,
        id: ConstID,
    },
    LoadStaticBigInt {
        result: Register,
        id: ConstID,
    },
    LoadStaticBigInt32 {
        result: Register,
        value: i32,
    },
    LoadTrue {
        result: Register,
    },
    LoadFalse {
        result: Register,
    },
    LoadNull {
        result: Register,
    },
    LoadUndefined {
        result: Register,
    },
    LoadThis {
        result: Register,
    },
    /// overide the this value with value
    SetThis {
        value: Register,
    },

    /// create a normal object
    CreateObject {
        result: Register,
    },
    /// create array, read elements from temp alloc
    CreateArray {
        result: Register,
    },
    /// create an arrow function, captures the this value
    CreateArrow {
        result: Register,
        this: Register,
        id: FuncID,
    },
    CreateFunction {
        result: Register,
        id: FuncID,
    },
    CreateClass {
        result: Register,
        class_id: ClassID,
    },
    /// create regexp
    CreateRegExp {
        result: Register,
        reg_id: RegexID,
    },
    /// arguments are created as same as function call
    CreateTemplate {
        result: Register,
        id: TemplateID,
        stack_offset:u16,   
    },
    /// arguments are created as same as function call
    CreateTaggedTemplate{
        result:Register,
        id: TemplateID,
        stack_offset:u16,
    },

    /// set the super value of a class
    ClassBindSuper {
        class: Register,
        super_: Register,
    },

    /// deep clone an object
    CloneObject {
        obj: Register,
        result: Register,
    },
}

#[test]
fn test_opcode_size() {
    assert!(std::mem::size_of::<OpCode>() == 8)
}
