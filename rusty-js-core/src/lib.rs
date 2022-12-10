pub mod baseline;
pub mod bultins;
pub mod bytecodes;
mod interpreter;
//mod clousure_jit;
pub mod runtime;
pub mod types;

pub mod convertion;
mod ecma;
mod error;
//mod fast_iter;
mod operations;
mod utils;

mod debug;

mod type_script;

pub use runtime::{ClassID, ConstID, FuncID, RegexID, Runtime, StringID, TemplateID};

pub use bultins::{
    bigint::JSBigInt, function::JSContext, object::CustomObject, object::JObject, object::PropKey,
    object::ToProperyKey, object_builder::ObjectBuilder, promise::Promise, proxy::Proxy, regex,
    strings::JSString, symbol::JSymbol, typed_array::TypedArray,
};

pub use types::JValue;
