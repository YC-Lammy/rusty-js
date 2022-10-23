pub mod baseline;
pub mod bultins;
pub mod bytecodes;
mod interpreter;
mod parser;
pub mod runtime;
pub mod types;

pub mod convertion;
mod ecma;
mod error;
mod fast_iter;
mod operations;
mod utils;

mod debug;

mod type_script;

pub use runtime::{ClassID, ConstID, FuncID, RegexID, Runtime, StringID, TemplateID};

pub use bultins::{
    function::JSFuncContext, object::JObject, object_builder::ObjectBuilder, promise::Promise,
    proxy::Proxy, regex, strings::JString, symbol::JSymbol, typed_array::TypedArray,
};
