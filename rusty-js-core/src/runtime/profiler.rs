use std::collections::HashMap;

use crate::types::JTypeVtable;

pub type BytecodeLine = u64;
pub struct Profiler {
    current:Option<&'static [*const JTypeVtable]>,
    results: Vec<&'static [*const JTypeVtable]>,
}

impl Profiler {
    pub fn speculate(&mut self, line: BytecodeLine, tys: ()) {}
}

impl Default for Profiler{
    fn default() -> Self {
        Self { 
            current:None,
            results: Default::default() 
        }
    }
}