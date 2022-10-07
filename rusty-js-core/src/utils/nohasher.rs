use std::hash::{BuildHasherDefault, Hasher};

pub type NoHasherBuilder = BuildHasherDefault<NoHasher>;

pub struct NoHasher {
    value: u64,
}

impl Hasher for NoHasher {
    fn finish(&self) -> u64 {
        return self.value;
    }

    fn write(&mut self, bytes: &[u8]) {
        unimplemented!("nohasher does not support bytes.")
    }

    fn write_i128(&mut self, i: i128) {
        self.value = i as u64
    }

    fn write_i16(&mut self, i: i16) {
        self.value = i as u64
    }

    fn write_i32(&mut self, i: i32) {
        self.value = i as u64
    }

    fn write_i64(&mut self, i: i64) {
        self.value = i as u64
    }

    fn write_i8(&mut self, i: i8) {
        self.value = i as u64
    }

    fn write_isize(&mut self, i: isize) {
        self.value = i as u64
    }

    fn write_u128(&mut self, i: u128) {
        self.value = i as u64
    }

    fn write_u16(&mut self, i: u16) {
        self.value = i as u64
    }

    fn write_u32(&mut self, i: u32) {
        self.value = i as u64
    }

    fn write_u64(&mut self, i: u64) {
        self.value = i as u64
    }

    fn write_u8(&mut self, i: u8) {
        self.value = i as u64
    }

    fn write_usize(&mut self, i: usize) {
        self.value = i as u64
    }
}

impl Default for NoHasher {
    fn default() -> Self {
        Self { value: 0 }
    }
}
