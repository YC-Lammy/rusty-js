use std::ops;

#[derive(Debug, Clone, Copy)]
pub struct PropFlag(u8);

impl PropFlag {
    pub const ENUMERABLE: PropFlag = PropFlag(0b00000001);
    pub const WRITABLE: PropFlag = PropFlag(0b00000010);
    pub const CONFIGURABLE: PropFlag = PropFlag(0b00000100);
    pub const GETTER: PropFlag = PropFlag(0b00001000);
    pub const SETTER: PropFlag = PropFlag(0b00010000);

    /// WRITABLE and CONFIGURABLE
    pub const BUILTIN: PropFlag = PropFlag(0b00000010 | 0b00000100);

    ///
    pub const THREE: PropFlag = PropFlag(0b00000001 | 0b00000010 | 0b00000100);

    pub fn is_enumerable(self) -> bool {
        return (self & Self::ENUMERABLE).0 != 0;
    }

    pub fn is_writable(self) -> bool {
        return (self & Self::WRITABLE).0 != 0;
    }

    pub fn is_configurable(self) -> bool {
        return (self & Self::CONFIGURABLE).0 != 0;
    }

    pub fn is_getter(self) -> bool {
        return (self & Self::GETTER).0 != 0;
    }

    pub fn is_setter(self) -> bool {
        return (self & Self::SETTER).0 != 0;
    }
}

impl Default for PropFlag {
    fn default() -> Self {
        PropFlag::THREE
    }
}

impl ops::BitOr for PropFlag {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self::Output {
        return PropFlag(self.0 | rhs.0);
    }
}

impl ops::BitAnd for PropFlag {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self::Output {
        return PropFlag(self.0 & rhs.0);
    }
}
