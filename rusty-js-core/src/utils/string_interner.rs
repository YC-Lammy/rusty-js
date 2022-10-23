use std::collections::HashMap;

use lazy_static::__Deref;
use string_interner::Symbol;

lazy_static::lazy_static!{
    pub static ref NAMES:HashMap<&'static str, u32> = Default::default();
}

lazy_static::lazy_static!{
    pub static ref INTERNER:string_interner::StringInterner = {
        let mut s = string_interner::StringInterner::new();
        init_names(&mut s);
        s
    };
}

fn init_names(int:&mut string_interner::StringInterner){
    let names = unsafe{(NAMES.deref() as *const _ as *mut HashMap<&'static str, u32>).as_mut().unwrap()};
    register_name("Object", int, names);
    register_name("__proto__", int, names);
    register_name("constructor", int, names);
    register_name("assign", int, names);
    register_name("create", int, names);
    register_name("defineProperty", int, names);
    register_name("defineProperties", int, names);
    register_name("entries", int, names);
    register_name("freeze", int, names);
    register_name("fromEntries", int, names);
    register_name("getOwnPropertyDescriptor", int, names);
    register_name("getOwnPropertyDescriptors", int, names);
    register_name("getOwnPropertyNames", int, names);
    register_name("getOwnPropertySymbols", int, names);
    register_name("getPrototypeOf", int, names);
    register_name("is", int, names);
    register_name("isExtensible", int, names);
    register_name("isFrozen", int, names);
    register_name("isSealed", int, names);
    register_name("keys", int, names);
    register_name("preventExtensions", int, names);
    register_name("seal", int, names);
    register_name("setPrototypeOf", int, names);
    register_name("values", int, names);
    register_name("__definedGetter__", int, names);
    register_name("__defineSetter__", int, names);
    register_name("__lookupGetter__", int, names);
    register_name("__lookupSetter__", int, names);
    register_name("hasOwnProperty", int, names);
    register_name("isPrototypeOf", int, names);
    register_name("propertyIsEnumerable", int, names);
    register_name("toLocaleString", int, names);
    register_name("toString", int, names);
    register_name("valueOf", int, names);

    register_name("Number", int, names);
    register_name("EPSILON", int, names);
    register_name("MAX_SAFE_INTEGER", int, names);
    register_name("MAX_VALUE", int, names);
    register_name("MIN_SAFE_INTEGER", int, names);
    register_name("MIN_VALUE", int, names);
    register_name("NaN", int, names);
    register_name("NEGATIVE_INFINITY", int, names);
    register_name("POSITIVE_INFINITY", int, names);
    register_name("isNaN", int, names);
    register_name("isFinite", int, names);
    register_name("isInteger", int, names);
    register_name("isSafeInteger", int, names);
    register_name("parseFloat", int, names);
    register_name("parseInt", int, names);
    register_name("toExponential", int, names);
    register_name("toFixed", int, names);
    register_name("toPrecision", int, names);
}

fn register_name(name:&'static str, int:&mut string_interner::StringInterner, names:&mut HashMap<&'static str, u32>){
    names.insert(name, int.get_or_intern_static(name).to_usize() as u32);
}