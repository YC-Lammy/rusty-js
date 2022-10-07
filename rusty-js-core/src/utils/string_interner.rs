use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use super::nohasher::NoHasherBuilder;

pub struct InternerSymbol(pub u64);

pub struct StringInterner {
    inner: HashMap<u64, String, NoHasherBuilder>,
}

impl StringInterner {
    pub fn get_or_intern<S>(&mut self, s: S) -> InternerSymbol
    where
        S: Into<String> + Hash + Eq,
    {
        let mut h = DefaultHasher::new();
        s.hash(&mut h);
        let r = h.finish();
        if !self.inner.contains_key(&r) {
            self.inner.insert(r, s.into());
        }
        return InternerSymbol(r);
    }
}
