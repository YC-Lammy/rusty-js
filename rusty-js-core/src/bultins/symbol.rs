use std::collections::HashMap;

use lock_api::RwLock as ApiLock;
use parking_lot::RwLock;
use string_interner::DefaultBackend;
use string_interner::StringInterner;

lazy_static::lazy_static! {
    static ref SYMBOL_INTERNER:RwLock<StringInterner<DefaultBackend<usize>>> = RwLock::new(StringInterner::new());
    static ref NAME_RESOLVER:RwLock<HashMap<u32, usize>> = RwLock::new(Default::default());
}

#[cfg(target_pointer_width = "64")]
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct JSymbol {
    id: u32,
    name: u32,
}

#[cfg(target_pointer_width = "32")]
#[derive(Debug, Clone, Copy)]
pub struct JSymbol {
    id: u32,
}

impl JSymbol {
    fn name(&self) -> &str {
        #[cfg(target_pointer_width = "64")]
        if self.name == 0 {
            self.to_mut().name = NAME_RESOLVER.read()[&self.id] as u32;
        }
        #[cfg(target_pointer_width = "64")]
        let key = self.name as usize;
        #[cfg(target_pointer_width = "32")]
        let key = NameResolver.read()[&self.id];

        let lock = ApiLock::read(&SYMBOL_INTERNER);
        let v = lock.resolve(key as usize).unwrap();
        unsafe { std::mem::transmute_copy(&v) }
    }

    fn to_mut(&self) -> &mut Self {
        unsafe { (self as *const _ as *mut Self).as_mut().unwrap() }
    }
}

impl ToString for JSymbol {
    fn to_string(&self) -> String {
        return format!("Symbol({})", self.name());
    }
}
