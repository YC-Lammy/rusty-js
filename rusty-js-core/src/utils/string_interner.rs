use std::{borrow::Cow, collections::HashMap};

use lazy_static::__Deref;

use crate::PropKey;

use super::nohasher::NoHasherBuilder;

lazy_static::lazy_static! {
    pub static ref NAMES:HashMap<&'static str, PropKey> = Default::default();
    pub static ref SYMBOLS:HashMap<&'static str, PropKey> = Default::default();
}

lazy_static::lazy_static! {
    pub static ref INTERNER:StringInterner = {
        let mut s = StringInterner::new();
        init_names(&mut s);
        s
    };
}

#[derive(Default, Clone)]
pub struct StringInterner {
    // already hashed, use the nohasher
    indexes: HashMap<u64, usize, NoHasherBuilder>,
    strings: Vec<Cow<'static, str>>,
}

impl StringInterner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_or_intern_static(&mut self, string: &'static str) -> usize {
        let hash = fxhash::hash64(&string);

        if let Some(v) = self.indexes.get(&hash) {
            return *v;
        }
        let idx = self.strings.len();
        self.strings.push(Cow::Borrowed(string));
        self.indexes.insert(hash, idx);
        return idx;
    }

    pub fn get_or_intern<S>(&mut self, string: S) -> usize
    where
        S: Into<String>,
    {
        let string = string.into();
        let hash = fxhash::hash64(&string);

        if let Some(i) = self.indexes.get(&hash) {
            return *i;
        }

        let idx = self.strings.len();
        self.strings.push(Cow::Owned(string));
        self.indexes.insert(hash, idx);
        return idx;
    }

    pub fn resolve(&self, idx: usize) -> Option<&str> {
        self.strings.get(idx).and_then(|v| Some(v.as_ref()))
    }

    pub fn reserve(&mut self) -> usize {
        let idx = self.strings.len();
        self.strings.push(Cow::Borrowed(""));
        idx
    }
}

// static names known at compile time
fn init_names(int: &mut StringInterner) {
    let names = unsafe {
        (NAMES.deref() as *const _ as *mut HashMap<&'static str, u32>)
            .as_mut()
            .unwrap()
    };
    let symbols = unsafe {
        (SYMBOLS.deref() as *const _ as *mut HashMap<&'static str, u32>)
            .as_mut()
            .unwrap()
    };

    symbols.insert("asyncIterator", int.reserve() as u32);
    symbols.insert("hasInstance", int.reserve() as u32);
    symbols.insert("isConcatSpreadable", int.reserve() as u32);
    symbols.insert("iterator", int.reserve() as u32);
    symbols.insert("match", int.reserve() as u32);
    symbols.insert("matchAll", int.reserve() as u32);
    symbols.insert("replace", int.reserve() as u32);
    symbols.insert("search", int.reserve() as u32);
    symbols.insert("split", int.reserve() as u32);
    symbols.insert("species", int.reserve() as u32);
    symbols.insert("toPrimitive", int.reserve() as u32);
    symbols.insert("toStringTag", int.reserve() as u32);
    symbols.insert("unscopables", int.reserve() as u32);

    let name = include_str!("names.txt");

    for name in name.lines() {
        register_name(name, int, names);
    }
}

fn register_name(
    name: &'static str,
    int: &mut StringInterner,
    names: &mut HashMap<&'static str, u32>,
) {
    names.insert(name, int.get_or_intern_static(name) as u32);
}
