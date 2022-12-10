use std::collections::HashMap;

use lock_api::RwLock as ApiLock;
use parking_lot::RwLock;

use crate::utils::string_interner::StringInterner;

lazy_static::lazy_static! {
    static ref SYMBOL_INTERNER:RwLock<StringInterner> = RwLock::new(StringInterner::new());
    static ref NAME_RESOLVER:RwLock<HashMap<u32, usize>> = RwLock::new(Default::default());
}

#[cfg(target_pointer_width = "64")]
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct JSymbol(pub u32);

impl JSymbol {
    fn to_mut(&self) -> &mut Self {
        unsafe { &mut *(self as *const _ as *mut Self) }
    }
}

impl ToString for JSymbol {
    fn to_string(&self) -> String {
        return format!("Symbol({})", self.0);
    }
}
