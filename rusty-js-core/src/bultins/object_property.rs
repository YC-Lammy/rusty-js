use std::{ops, collections::HashMap};

use crate::{Runtime, JValue, utils::nohasher::NoHasherBuilder};

#[derive(Debug, Clone, Copy)]
pub struct PropFlag(u8);

impl PropFlag {
    pub const NONE: PropFlag = PropFlag(0);
    pub const ENUMERABLE: PropFlag = PropFlag(0b00000001);
    pub const WRITABLE: PropFlag = PropFlag(0b00000010);
    pub const CONFIGURABLE: PropFlag = PropFlag(0b00000100);
    pub const GETTER: PropFlag = PropFlag(0b00001000);
    pub const SETTER: PropFlag = PropFlag(0b00010000);

    /// WRITABLE and CONFIGURABLE
    pub const BUILTIN: PropFlag = PropFlag(Self::WRITABLE.0 | Self::CONFIGURABLE.0);

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

#[derive(Hash, Clone, Copy, PartialEq, Eq)]
pub struct PropKey(pub(crate) u32);

pub trait ToProperyKey {
    fn to_key(&self, runtime: &Runtime) -> PropKey;
}

impl ToProperyKey for PropKey {
    fn to_key(&self, _runtime: &Runtime) -> PropKey {
        return *self;
    }
}

impl ToProperyKey for JValue {
    fn to_key(&self, runtime: &Runtime) -> PropKey {
        if let Some(s) = self.as_string() {
            let id = runtime.register_field_name(s.as_ref());
            return PropKey(id);
        } else {
            let s = self.to_string();
            let id = runtime.register_field_name(&s);
            return PropKey(id);
        }
    }
}

impl<S> ToProperyKey for S
where
    S: AsRef<str>,
{
    fn to_key(&self, runtime: &Runtime) -> PropKey {
        let id = runtime.register_field_name(self.as_ref());
        PropKey(id)
    }
}

#[derive(Clone, Copy)]
pub struct PropCell {
    pub flag: PropFlag,
    /// value acts as getter when flag has getter
    pub value: JValue,
    pub setter: JValue,
}

pub type PropMap = HashMap<PropKey, PropCell, NoHasherBuilder>;
