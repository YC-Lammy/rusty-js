use std::sync::Arc;

use error::Error;
use op::Op;
use parser::ParseState;
use util::{DynamicBuffer, DynamicBufferIterator};

mod parser;
mod op;
mod util;
mod error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Flags(u8);

impl Flags{
    const HasIndices:Self = Flags(0b00000001);
    const Global:Self = Flags(0b00000010);
    const IgnoreCase:Self = Flags(0b00000100);
    const Multiline:Self = Flags(0b00001000);
    const DotAll:Self = Flags(0b00010000);
    const Unicode:Self = Flags(0b00100000);
    const Sticky:Self = Flags(0b01000000);
}

impl Flags{
    pub fn dotall(self) -> bool{
        self & Self::DotAll
    }

    pub fn ignore_case(self) -> bool{
        self & Self::IgnoreCase
    }

    pub fn has_indices(self) -> bool{
        self & Self::HasIndices
    }

    pub fn global(self) -> bool{
        self & Self::Global
    }

    pub fn multiline(self) -> bool{
        self & Self::Multiline
    }

    pub fn unicode(self) -> bool{
        self & Self::Unicode
    }

    pub fn sticky(self) -> bool{
        self & Self::Sticky
    }
}

impl std::ops::BitOr for Flags{
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl std::ops::BitOrAssign for Flags{
    fn bitor_assign(&mut self, rhs: Self) {
        *self = Self(self.0 | rhs.0)
    }
}

impl std::ops::BitAnd for Flags{
    type Output = bool;
    fn bitand(self, rhs: Self) -> Self::Output {
        (self.0 & rhs.0) != 0
    }
}

#[derive(Clone)]
pub struct RegExp{
    inner:Arc<RegExpInner>
}

pub struct RegExpInner{
    flags:Flags,
    num_captures:u32,
    group_names:Vec<Option<String>>,
    stack_size:u32,
    bytecode:DynamicBuffer,

    stack:Vec<u32>,
    exec_state:ExecState,
    count:usize,

}

impl RegExpInner{
    pub fn compile(pattern:&str, flags:Flags) -> Result<Self, Error>{
        let pat = pattern.to_string() + "\x00";
        let mut state = ParseState{
            pattern:pat.as_bytes(),
            pattern_ptr:pat.as_bytes(),
            bytecode:DynamicBuffer::new(),
            ignore_case:flags.ignore_case(),
            is_utf16:false,
            flag:flags,
            capture_count:1,
            total_capture_count:-1,
            has_named_captures:-1,
            group_names:Vec::new(),
            tmp_buf:Vec::new(),
            error:None,
            opaque:0 as *const ()
        };

        if !flags.sticky(){
            state.bytecode.push_u8(Op::SplitGotoFirst as u8);
            state.bytecode.push_u32(1 + 5);
            state.bytecode.push_u8(Op::Any as u8);
            state.bytecode.push_u8(Op::Goto as u8);
            state.bytecode.push_u32(-(5i32 + 1 + 5) as u32);
        }
        state.bytecode.push_u8(Op::SaveStart as u8);
        state.bytecode.push_u8(0);

        if state.parse_disjunction(false) != 0{
            return Err(state.error.unwrap())
        };

        state.bytecode.push_u8(Op::SaveEnd as u8);
        state.bytecode.push_u8(0);

        state.bytecode.push_u8(Op::Match as u8);

        assert!(state.pattern_ptr[0] == b'\x00');

        let stack_size = state.compute_stack_size();
        let mut stack = Vec::with_capacity(stack_size);
        stack.resize(stack_size, 0);

        return Ok(Self{
            flags:flags,
            num_captures:state.capture_count as u32,
            stack_size:stack_size as u32,
            group_names:state.group_names,
            bytecode:state.bytecode,

            stack:stack,

            // state
            exec_state:ExecState::Split,
            count:0,
        })
    }

    pub fn exec<P>(&mut self, mut string:P) -> Result<usize, ()> where P:Peekable{
        let mut index = 0;
        let mut captures = Vec::new();
        captures.resize(self.num_captures as usize, None);

        let no_recurse = false;

        let mut iter = self.bytecode.iter();
        
        let ret = self.exec_backtrace(&mut string, 0, &mut iter, false);
    }

    fn exec_backtrace<P>(&mut self, string:&mut P, mut string_offset:usize, iter:&mut DynamicBufferIterator, no_recurse:bool) -> Result<usize, ()> where P:Peekable{
        let mut ret = 0;

        loop{
            let op:Op = if let Some(v) = iter.get_next_u8(){
                unsafe{std::mem::transmute(v)}
            } else{
                return Ok(0);
            };

            match op{
                Op::Match => {
                    if no_recurse{
                        return Ok(string_offset)
                    }
                }
            }
        }
    }
}

enum ExecState{
    Split,
    Lookahead,
    NegativeLookahead,
    GreedyQuant
}

pub struct Match{
    pub range:std::ops::Range<usize>,
    pub captures:Vec<Option<std::ops::Range<usize>>>
}

pub trait Peekable{
    fn peek(&mut self, idx:usize) -> Option<char>;
}

impl Peekable for &[u8]{
    fn peek(&mut self, idx:usize) -> Option<char> {
        if self.len() <= idx{
            return None
        }

        Some(self[idx] as char)
    }
}

impl Peekable for &[char]{
    fn peek(&mut self, idx:usize) -> Option<char> {
        if self.len() <= idx{
            return None
        }

        Some(self[idx])
    }
}

impl Peekable for &[u16]{
    fn peek(&mut self, idx:usize) -> Option<char> {
        if self.len() <= idx{
            return None
        }

        char::from_u32(self[idx] as u32)
    }
}

impl Peekable for Vec<u8>{
    fn peek(&mut self, idx:usize) -> Option<char> {
        self.as_slice().peek(idx)
    }
}

impl Peekable for Vec<u16>{
    fn peek(&mut self, idx:usize) -> Option<char> {
        self.as_slice().peek(idx)
    }
}

impl Peekable for Vec<char>{
    fn peek(&mut self, idx:usize) -> Option<char> {
        self.as_slice().peek(idx)
    }
}

impl Peekable for &str{
    fn peek(&mut self, idx:usize) -> Option<char> {
        self.as_bytes().peek(idx)
    }
}

impl Peekable for String{
    fn peek(&mut self, idx:usize) -> Option<char> {
        self.as_bytes().peek(idx)
    }
}