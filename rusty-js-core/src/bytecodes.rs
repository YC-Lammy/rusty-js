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
pub struct Block(pub u32);

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

#[repr(packed)]
pub struct TempAllocValue {
    pub flag: u8,
    pub value: JValue,
}

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
    pub fn target(&self) -> Register {
        match self {
            Self::T0R0 | Self::T0R1 | Self::T0R2 | Self::T0R3 => Register(0),
            Self::T1R0 | Self::T1R1 | Self::T1R2 | Self::T1R3 => Register(1),
            Self::T2R0 | Self::T2R1 | Self::T2R2 | Self::T2R3 => Register(2),
            Self::T3R0 | Self::T3R1 | Self::T3R2 | Self::T3R3 => Register(3),
        }
    }

    pub fn value(&self) -> Register {
        match self {
            Self::T0R0 | Self::T1R0 | Self::T2R0 | Self::T3R0 => Register(0),
            Self::T0R1 | Self::T1R1 | Self::T2R1 | Self::T3R1 => Register(1),
            Self::T0R2 | Self::T1R2 | Self::T2R2 | Self::T3R2 => Register(2),
            Self::T0R3 | Self::T1R3 | Self::T2R3 | Self::T3R3 => Register(3),
        }
    }
}

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
/// to get the offset in bytes: offset * size_of JValue .
#[derive(Debug, Clone, Copy)]
pub enum OpCode {
    NoOp,
    Debugger,

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

    CreateBlock(Block),
    SwitchToBlock(Block),
    Jump {
        to: Block,
    },
    JumpIfTrue {
        value: Register,
        to: Block,
    },
    JumpIfFalse {
        value: Register,
        to: Block,
    },
    JumpIfIterDone {
        to: Block,
    },

    EnterTry {
        catch_block: Block,
    },
    ExitTry,

    Throw {
        value: Register,
    },

    Return {
        value: Register,
    },

    CreateArg {
        stack_offset: u16,
        len: u32,
    },
    PushArg {
        value: Register,
        stack_offset:u16,
    },
    PushArgSpread {
        value: Register,
        stack_offset:u16,
    },
    FinishArgs{
        base_stack_offset:u16,
        len:u16,
    },

    /// allocate memory and store in alloc register,
    /// this op is supposed to store values temporary
    /// and is expected to deallocate immediately.
    TempAlloc {
        /// size in bytes
        size: u32,
    },
    TempDealloc {
        /// size in bytes
        size: u32,
    },
    StoreTempAlloc {
        /// offset in bytes
        offset: u16,
        flag: u8,
        value: Register,
    },
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

    DeclareDynamicVar {
        from: Register,
        kind: DeclareKind,
        id: u32,
    },
    WriteDynamicVar {
        from: Register,
        id: u32,
    },
    ReadDynamicVar {
        result: Register,
        id: u32,
    },

    /// capture variable from stackoffset
    Capture {
        stack_offset: u16,
        capture_stack_offset: u16,
    },
    ReadCapturedVar {
        result: Register,
        offset: u16,
    },
    WriteCapturedVar {
        from: Register,
        offset: u16,
    },

    WriteToStack {
        from: Register,
        stack_offset: u16,
    },
    ReadFromStack {
        result: Register,
        stack_offset: u16,
    },
    ReadParam {
        result: Register,
        index: u32,
    },
    CollectParam {
        result: Register,
        start: u32,
    },

    /// the arguments are in the stack frame
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
    New {
        result: Register,
        callee: Register,
        stack_offset: u16,
    },
    NewTarget{
        result:Register
    },
    ImportMeta{
        result:Register
    },

    IntoIter {
        target: Register,
        hint: LoopHint,
    },
    IterNext {
        result: Register,
        hint: LoopHint,
        stack_offset: u16,
    },
    IterCollect {
        result: Register,
        stack_offset: u16,
    },
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
    WriteFieldStatic {
        obj_value: CompactRegister,
        field_id: u32,
        stack_offset: u16,
    },
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
    RemoveFieldStatic {
        obj: Register,
        field_id: u32,
    },

    BindSetter {
        obj: Register,
        field_id: u32,
        setter: Register,
    },
    BindGetter {
        obj: Register,
        field_id: u32,
        getter: Register,
    },
    /// extend an object by an object
    ExtendObject{
        obj:Register,
        from:Register,
    },
    ReadSuperField{
        constructor_result:CompactRegister,
        field:Register,
        stack_offset:u16,
    },
    ReadSuperFieldStatic{
        constructor_result:CompactRegister,
        field_id: u32,
        stack_offset:u16,
    },
    WriteSuperField{
        constructor_value:CompactRegister,
        field:Register,
        stack_offset:u16,
    },
    WriteSuperFieldStatic{
        constructor_value:CompactRegister, // 8bit
        field:u32, // 32 bit
        stack_offset:u16, // 16bit
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
    SetThis {
        value: Register,
    },

    CreateObject {
        result: Register,
    },
    /// create array, read elements from temp alloc
    CreateArray {
        result: Register,
    },
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
        tagged: bool,
    },

    ClassBindSuper {
        class: Register,
        super_: Register,
    },

    CloneObject {
        obj: Register,
        result: Register,
    },
}

#[test]
fn test_opcode_size() {
    assert!(std::mem::size_of::<OpCode>() == 8)
}
