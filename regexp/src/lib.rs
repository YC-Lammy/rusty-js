mod parser;
mod op;
mod util;

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