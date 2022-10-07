use crate::util::DynamicBufferIterator;


#[repr(u8)]
pub enum Op{
    Match,
    Char32,
    Char,
    SplitGotoFirst,
    SplitNextFirst,
    Lookahead,
    NegativeLookahead,
    Goto,
    LineStart,
    LineEnd,
    Dot,
    Any,
    SaveStart,
    SaveEnd,
    SaveReset,
    /// save u32 to stack
    PushU32,
    /// drop the last stack value
    Drop,
    Loop,
    PushCharPos,
    WordBoundary,
    NotWordBoundary,
    BackReference,
    BackwardBackReference,

    Ranges,
    Prev,
    SimpleGreedyQuant,

    BneCharPos
}

impl Op{
    pub fn consume_iter(&self, iter:&mut DynamicBufferIterator){
        match self{
            Op::Any |
            Op::Dot |
            Op::LineStart |
            Op::LineEnd |
            Op::Match |
            Op::Drop |
            Op::WordBoundary |
            Op::NotWordBoundary |
            Op::PushCharPos |
            Op::Prev => {},
            Op::Char => {iter.get_next_u16();},
            Op::Char32 => {iter.get_next_u32();},
            Op::Goto => {iter.get_next_u32();},
            Op::SplitGotoFirst => {iter.get_next_u32();},
            Op::SplitNextFirst => {iter.get_next_u32();},
            Op::SaveStart |
            Op::SaveEnd => {iter.get_next_u8();},
            Op::SaveReset => {
                iter.get_next_u8();
                iter.get_next_u8();
            },
            Op::Loop => {iter.get_next_u32();},
            Op::PushU32 => {iter.get_next_u32();},
            Op::BackReference => {iter.get_next_u8();},
            Op::BackwardBackReference => {iter.get_next_u8();},
            Op::Ranges => {
                let len = iter.get_next_u16().unwrap();
                for i in 0..len{
                    iter.get_next_u32().unwrap();
                    iter.get_next_u32().unwrap();
                    iter.get_next_bool().unwrap();
                };
            },
            Op::Lookahead => {iter.get_next_u32();},
            Op::NegativeLookahead => {iter.get_next_u32();},
            Op::BneCharPos => {iter.get_next_u32();},
            Op::SimpleGreedyQuant => {
                iter.get_next_u32();
                iter.get_next_u32();
                iter.get_next_u32();
                iter.get_next_u32();

            }
        
        };
    }
}

