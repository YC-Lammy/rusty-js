use std::{any::Any, ops::Range, sync::Arc};

use crate::types::JValue;

#[derive(Debug, Clone)]
pub enum Error {
    ExpectedFunction,
    CallOnNonFunction,
    FunctionCallArgumentsOverflow,
    ClassCannotBeInvokedWithoutNew,
    ClassExtendsNonCallable,
    LabelUndefined(String),

    IllegalBreak,
    IllegalContinue,

    InvalidExpression { pos: Range<u32> },
    InvalideIterator { msg: &'static str },

    AwaitOnForeverPendingPromise,
    ImportError(String),

    RuntimeError(String),
    TypeError(String),
    SyntaxError(String),
    ReferenceError(String),
    RangeError(String),
    Value(JValue),
    User(Arc<dyn Any>),
}

unsafe impl Sync for Error {}
unsafe impl Send for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self, f)
    }
}
