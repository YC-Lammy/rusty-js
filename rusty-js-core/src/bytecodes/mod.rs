use crate::runtime::{ClassID, ConstID, FuncID, RegexID, StringID, TemplateID};

pub mod function_builder_context;
pub mod bytecode_builder;
pub mod optimize;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Hash)]
pub struct Register(pub u8);

impl Into<usize> for Register {
    fn into(self) -> usize {
        self.0 as usize
    }
}

impl<'a> Into<usize> for &'a Register {
    fn into(self) -> usize {
        self.0 as usize
    }
}

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
///
use rusty_js_macros::ByteCode;

pub trait ByteCode {
    fn writes_to(self) -> Option<Register>;
    fn reads_from(self) -> Vec<Register>;
}

#[derive(ByteCode, Debug, Clone, Copy)]
pub enum OpCode {
    /// no op
    NoOp,
    /// call the debugger, if the debugger on runtime is not set,
    /// this is a no op
    Debugger,

    /// mov from register to another,
    /// if the two registers are the same, this is a no op
    Mov {
        #[r]
        from: Register,
        #[w]
        to: Register,
    },

    /// return b if a is undefined, else a
    Select {
        #[r]
        a: Register,
        #[w]
        b: Register,
        #[w]
        result: Register,
    },
    /// return a if t is true, else b
    CondSelect {
        #[r]
        t: Register,
        #[r]
        a: Register,
        #[r]
        b: Register,
        #[w]
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
        line: u32,
    },
    JumpIfTrue {
        #[r]
        value: Register,
        to: Block,
        line: u32,
    },
    JumpIfFalse {
        #[r]
        value: Register,
        to: Block,
        line: u32,
    },

    /// enter a try context, when an error occours, will jump to the catch block
    EnterTry {
        catch_block: Block,
        line: u32,
    },
    /// finish try
    ExitTry,

    /// throw and cause a panic
    Throw {
        #[r]
        value: Register,
    },

    /// return the value
    Return {
        #[r]
        value: Register,
    },

    /// create a list of arguments on stack
    CreateArg {
        stack_offset: u16,
        len: u32,
    },
    /// write an argument to its location on stack
    PushArg {
        #[r]
        value: Register,
        stack_offset: u16,
    },
    /// write an argument that needs to be spread
    PushArgSpread {
        #[r]
        value: Register,
        stack_offset: u16,
    },
    SpreadArg {
        base_stack_offset: u16,
        stack_offset: u16,
        args_len: u16,
    },
    /// finish the creation of arguments,
    ///
    /// if spread is required for any argument, spread it now
    FinishArgs {
        base_stack_offset: u16,
        len: u16,
    },

    /// create a temporary slot
    StoreTemp {
        #[r]
        value: Register,
    },
    /// read from current temporary slot
    ReadTemp {
        #[w]
        value: Register,
    },
    /// release current temporary slot
    ReleaseTemp,

    /// declared a dynamic variable
    DeclareDynamicVar {
        #[r]
        from: Register,
        kind: DeclareKind,
        offset: u16,
    },
    /// write to a dynamic variable,
    ///
    /// if the variable is not declared, this will write to global object
    WriteDynamicVar {
        #[r]
        from: Register,
        id: u32,
    },
    /// write to a global variable,
    WriteDynamicVarDeclared {
        #[r]
        from: Register,
        offset: u16,
    },

    /// read from a dynamic variable
    ///
    /// if the variable is not declared, this will read from gloabl object
    ReadDynamicVar {
        #[w]
        result: Register,
        id: u32,
    },

    /// read from a global variable
    ReadDynamicVarDeclared {
        #[w]
        result: Register,
        offset: u16,
    },

    /// capture variable from stack.
    ///
    /// the bytecode builder will no longer use the stack offset as variable,
    /// read and write of the dedicated variable will not be done on stack,
    /// but on the capture environment.
    //Capture {
    //    stack_offset: u16,
    //    capture_stack_offset: u16,
    //},
    /// read a variable from the capture environment
    ReadCapturedVar {
        #[w]
        result: Register,
        offset: u16,
    },
    /// write a variable to the capture environment
    WriteCapturedVar {
        #[r]
        from: Register,
        offset: u16,
    },

    /// write to the stack
    WriteToStack {
        #[r]
        from: Register,
        stack_offset: u16,
    },
    /// read from the stack
    ReadFromStack {
        #[w]
        result: Register,
        stack_offset: u16,
    },
    /// read a param
    ReadParam {
        #[w]
        result: Register,
        index: u32,
    },
    /// collect all the params to an array starting from index.
    CollectParam {
        #[w]
        result: Register,
        /// the starting index
        start: u32,
    },

    /// prepare for an inlined call
    PrepareInlinedCall {
        stack_offset: u16,
    },

    /// call a function using the stack + offset.
    ///
    /// arguments are already on the stack
    Call {
        #[w]
        result: Register,
        #[r]
        this: Register,
        #[r]
        callee: Register,
        stack_offset: u16,
        args_len: u16
    },

    /// invoke a new operation
    New {
        #[w]
        result: Register,
        #[r]
        callee: Register,
        stack_offset: u16,
    },
    /// get the metadata new.target
    NewTarget {
        #[w]
        result: Register,
    },
    /// get the metadata import.meta
    ImportMeta {
        #[w]
        result: Register,
    },

    PrepareForIn{
        #[r]
        target: Register
    },
    PrepareForOf{
        #[r]
        target: Register
    },
    /// get the next value from the iterator
    IterNext {
        #[w]
        result: Register,
        #[w]
        done: Register,
        hint: LoopHint,
        stack_offset: u16,
    },
    /// collect all the unread values from the iterator
    IterCollect {
        #[w]
        result: Register,
        stack_offset: u16,
    },
    /// destroy the current iterator and restore the preveous one
    IterDrop,

    IsNullish{
        #[w]
        result: Register,
        #[r]
        value: Register,
    },

    Add {
        #[w]
        result: Register,
        #[r]
        left: Register,
        #[r]
        right: Register,
    },
    Sub {
        #[w]
        result: Register,
        #[r]
        left: Register,
        #[r]
        right: Register,
    },
    Mul {
        #[w]
        result: Register,
        #[r]
        left: Register,
        #[r]
        right: Register,
    },
    Div {
        #[w]
        result: Register,
        #[r]
        left: Register,
        #[r]
        right: Register,
    },
    Exp {
        #[w]
        result: Register,
        #[r]
        left: Register,
        #[r]
        right: Register,
    },
    LShift {
        #[w]
        result: Register,
        #[r]
        left: Register,
        #[r]
        right: Register,
    },
    RShift {
        #[w]
        result: Register,
        #[r]
        left: Register,
        #[r]
        right: Register,
    },
    ZeroFillRShift {
        #[w]
        result: Register,
        #[r]
        left: Register,
        #[r]
        right: Register,
    },
    Rem {
        #[w]
        result: Register,
        #[r]
        left: Register,
        #[r]
        right: Register,
    },

    Await {
        #[w]
        result: Register,
        #[r]
        future: Register,
    },
    Yield {
        #[w]
        result: Register,
        #[r]
        arg: Register,
    },

    And {
        #[w]
        result: Register,
        #[r]
        left: Register,
        #[r]
        right: Register,
    },
    Or {
        #[w]
        result: Register,
        #[r]
        left: Register,
        #[r]
        right: Register,
    },
    Not {
        #[w]
        result: Register,
        #[r]
        right: Register,
    },
    BitNot {
        #[w]
        result: Register,
        #[r]
        right: Register,
    },
    BitAnd {
        #[w]
        result: Register,
        #[r]
        left: Register,
        #[r]
        right: Register,
    },
    BitOr {
        #[w]
        result: Register,
        #[r]
        left: Register,
        #[r]
        right: Register,
    },
    BitXor {
        #[w]
        result: Register,
        #[r]
        left: Register,
        #[r]
        right: Register,
    },
    Minus {
        #[w]
        result: Register,
        #[r]
        right: Register,
    },
    Plus {
        #[w]
        result: Register,
        #[r]
        right: Register,
    },
    /// return null if not null
    Nullish {
        #[w]
        result: Register,
        #[r]
        left: Register,
        #[r]
        right: Register,
    },

    EqEq {
        #[w]
        result: Register,
        #[r]
        left: Register,
        #[r]
        right: Register,
    },
    EqEqEq {
        #[w]
        result: Register,
        #[r]
        left: Register,
        #[r]
        right: Register,
    },
    NotEq {
        #[w]
        result: Register,
        #[r]
        left: Register,
        #[r]
        right: Register,
    },
    NotEqEq {
        #[w]
        result: Register,
        #[r]
        left: Register,
        #[r]
        right: Register,
    },
    Gt {
        #[w]
        result: Register,
        #[r]
        left: Register,
        #[r]
        right: Register,
    },
    GtEq {
        #[w]
        result: Register,
        #[r]
        left: Register,
        #[r]
        right: Register,
    },
    Lt {
        #[w]
        result: Register,
        #[r]
        left: Register,
        #[r]
        right: Register,
    },
    LtEq {
        #[w]
        result: Register,
        #[r]
        left: Register,
        #[r]
        right: Register,
    },

    AddImmI32 {
        #[w]
        result: Register,
        #[r]
        left: Register,
        right: i32,
    },
    AddImmF32 {
        #[w]
        result: Register,
        #[r]
        left: Register,
        right: f32,
    },
    AddImmStr {
        #[w]
        result: Register,
        #[r]
        left: Register,
        str: StringID,
    },
    SubImmI32 {
        #[w]
        result: Register,
        #[r]
        left: Register,
        right: i32,
    },
    SubImmF32 {
        #[w]
        result: Register,
        #[r]
        left: Register,
        right: f32,
    },
    MulImmI32 {
        #[w]
        result: Register,
        #[r]
        left: Register,
        right: i32,
    },
    MulImmF32 {
        #[w]
        result: Register,
        #[r]
        left: Register,
        right: f32,
    },
    DivImmI32 {
        #[w]
        result: Register,
        #[r]
        left: Register,
        right: i32,
    },
    DivImmF32 {
        #[w]
        result: Register,
        #[r]
        left: Register,
        right: f32,
    },
    ExpImmI32 {
        #[w]
        result: Register,
        #[r]
        left: Register,
        right: i32,
    },
    ExpImmF32 {
        #[w]
        result: Register,
        #[r]
        left: Register,
        right: f32,
    },
    LShiftImmI32 {
        #[w]
        result: Register,
        #[r]
        left: Register,
        right: i32,
    },
    RShiftImmI32 {
        #[w]
        result: Register,
        #[r]
        left: Register,
        right: i32,
    },
    ZeroFillRShiftImmI32 {
        #[w]
        result: Register,
        #[r]
        left: Register,
        right: i32,
    },
    RemImmI32 {
        #[w]
        result: Register,
        #[r]
        left: Register,
        right: i32,
    },
    RemImmF32 {
        #[w]
        result: Register,
        #[r]
        left: Register,
        right: f32,
    },

    AndImm {
        #[w]
        result: Register,
        #[r]
        left: Register,
        right: bool,
    },
    BitAndImmI32 {
        #[w]
        result: Register,
        #[r]
        left: Register,
        right: i32,
    },
    BitOrImmI32 {
        #[w]
        result: Register,
        #[r]
        left: Register,
        right: i32,
    },
    BitXorImmI32 {
        #[w]
        result: Register,
        #[r]
        left: Register,
        right: i32,
    },

    EqEqImmI32 {
        #[w]
        result: Register,
        #[r]
        left: Register,
        right: i32,
    },
    EqEqImmF32 {
        #[w]
        result: Register,
        #[r]
        left: Register,
        right: f32,
    },
    EqEqEqImmI32 {
        #[w]
        result: Register,
        #[r]
        left: Register,
        right: i32,
    },
    EqEqEqImmF32 {
        #[w]
        result: Register,
        #[r]
        left: Register,
        right: f32,
    },
    NotEqImmI32 {
        #[w]
        result: Register,
        #[r]
        left: Register,
        right: i32,
    },
    NotEqImmF32 {
        #[w]
        result: Register,
        #[r]
        left: Register,
        right: f32,
    },
    NotEqEqImmI32 {
        #[w]
        result: Register,
        #[r]
        left: Register,
        right: i32,
    },
    NotEqEqImmF32 {
        #[w]
        result: Register,
        #[r]
        left: Register,
        right: f32,
    },
    GtImmI32 {
        #[w]
        result: Register,
        #[r]
        left: Register,
        right: i32,
    },
    GtImmF32 {
        #[w]
        result: Register,
        #[r]
        left: Register,
        right: f32,
    },
    GtEqImmI32 {
        #[w]
        result: Register,
        #[r]
        left: Register,
        right: i32,
    },
    GtEqImmF32 {
        #[w]
        result: Register,
        #[r]
        left: Register,
        right: f32,
    },
    LtImmI32 {
        #[w]
        result: Register,
        #[r]
        left: Register,
        right: i32,
    },
    LtImmF32 {
        #[w]
        result: Register,
        #[r]
        left: Register,
        right: f32,
    },
    LtEqImmI32 {
        #[w]
        result: Register,
        #[r]
        left: Register,
        right: i32,
    },
    LtEqImmF32 {
        #[w]
        result: Register,
        #[r]
        left: Register,
        right: f32,
    },

    In {
        #[w]
        result: Register,
        #[r]
        left: Register,
        #[r]
        right: Register,
    },
    PrivateIn {
        #[w]
        result: Register,
        #[r]
        right: Register,
        name: u32,
    },
    InstanceOf {
        #[w]
        result: Register,
        #[r]
        left: Register,
        #[r]
        right: Register,
    },
    TypeOf {
        #[w]
        result: Register,
        #[r]
        right: Register,
    },

    /// write the value of field that is loaded into register
    WriteField {
        #[r]
        obj: Register,
        #[r]
        field: Register,
        #[r]
        value: Register,
        stack_offset: u16,
    },
    ReadField {
        #[r]
        obj: Register,
        #[r]
        field: Register,
        #[w]
        result: Register,
        stack_offset: u16,
    },
    /// write the field of an object with a static field name
    WriteFieldStatic {
        #[r]
        obj: Register,
        #[r]
        value: Register,
        field_id: u32,
    },
    /// read the field of an object with a static field name
    ReadFieldStatic {
        #[r]
        obj: Register,
        #[w]
        result: Register,
        field_id: u32,
    },
    /// remove a field from an object
    RemoveFieldStatic {
        #[r]
        obj: Register,
        field_id: u32,
    },

    /// bind a setter into an object
    BindSetter {
        #[r]
        obj: Register,
        field_id: u32,
        #[r]
        setter: Register,
    },
    /// bind a getter into an object
    BindGetter {
        #[r]
        obj: Register,
        field_id: u32,
        #[r]
        getter: Register,
    },
    /// extend an object by an object
    ExtendObject {
        #[r]
        obj: Register,
        #[r]
        from: Register,
    },
    /// read from field of the super object
    ReadSuperField {
        #[w]
        result: Register,
        #[r]
        constructor: Register,
        #[r]
        field: Register,
        stack_offset: u16,
    },
    ReadSuperFieldStatic {
        #[w]
        result: Register,
        #[r]
        constructor: Register,
        field_id: u32,
    },
    WriteSuperField {
        #[r]
        constructor: Register,
        #[r]
        value: Register,
        field: Register,
    },
    WriteSuperFieldStatic {
        #[r]
        constructor: Register,
        #[r]
        value: Register, // 8bit
        field: u32, // 32 bit
    },

    LoadStaticString {
        #[w]
        result: Register,
        id: StringID,
    },
    LoadStaticFloat32 {
        #[w]
        result: Register,
        value: f32,
    },
    LoadStaticFloat {
        #[w]
        result: Register,
        id: ConstID,
    },
    LoadStaticBigInt {
        #[w]
        result: Register,
        id: ConstID,
    },
    LoadStaticBigInt32 {
        #[w]
        result: Register,
        value: i32,
    },
    LoadTrue {
        #[w]
        result: Register,
    },
    LoadFalse {
        #[w]
        result: Register,
    },
    LoadNull {
        #[w]
        result: Register,
    },
    LoadUndefined {
        #[w]
        result: Register,
    },
    LoadThis {
        #[w]
        result: Register,
    },
    /// overide the this value with value
    SetThis {
        #[r]
        value: Register,
    },

    /// create a normal object
    CreateObject {
        #[w]
        result: Register,
    },
    /// create array, read elements from temp alloc
    CreateArray {
        #[w]
        result: Register,
        stack_offset: u16,
    },
    /// create an arrow function, captures the this value
    CreateArrow {
        #[w]
        result: Register,
        #[r]
        this: Register,
        id: FuncID,
    },
    CreateFunction {
        #[w]
        result: Register,
        id: FuncID,
    },
    CreateClass {
        #[w]
        result: Register,
        class_id: ClassID,
    },
    /// create regexp
    CreateRegExp {
        #[w]
        result: Register,
        reg_id: RegexID,
    },
    /// arguments are created as same as function call
    CreateTemplate {
        #[w]
        result: Register,
        id: TemplateID,
        stack_offset: u16,
    },

    /// set the super value of a class
    ClassBindSuper {
        #[r]
        class: Register,
        #[r]
        super_: Register,
    },

    /// deep clone an object
    CloneObject {
        #[r]
        obj: Register,
        #[w]
        result: Register,
    },
}

#[test]
fn test(){
    let op = OpCode::Add { result: Register(0), left: Register(0), right: Register(0) };
    let data:[u8;8] = unsafe{std::mem::transmute(op)};
    println!("{}", data[0])
}