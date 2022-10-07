use std::collections::HashMap;

pub type BytecodeLine = u64;
pub struct Profiler {
    results: HashMap<BytecodeLine, Vec<()>>,
}

impl Profiler {
    pub fn speculate(&mut self, line: BytecodeLine, tys: ()) {}
}
