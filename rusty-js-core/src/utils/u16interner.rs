use std::collections::HashMap;

use super::nohasher::NoHasherBuilder;

#[derive(Default, Clone)]
pub struct U16Interner {
    // already hashed, use the nohasher
    indexes: HashMap<u64, usize, NoHasherBuilder>,
    data: Vec<Box<[u16]>>,
}

impl U16Interner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_or_intern(&mut self, bytes: Box<[u16]>) -> usize {
        let hash = fxhash::hash64(&bytes);

        if let Some(i) = self.indexes.get(&hash) {
            return *i;
        }

        let idx = self.data.len();
        self.data.push(bytes);
        self.indexes.insert(hash, idx);
        return idx;
    }

    pub fn resolve(&self, idx: usize) -> Option<&[u16]> {
        self.data.get(idx).and_then(|v| Some(v.as_ref()))
    }

    pub fn reserve(&mut self) -> usize {
        let idx = self.data.len();
        self.data.push(Box::new([0u16; 0]));
        idx
    }
}
