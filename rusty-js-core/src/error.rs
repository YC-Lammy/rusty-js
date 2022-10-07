use std::{any::Any, fmt::Pointer, ops::Range, sync::Arc};

use crate::types::JValue;

#[derive(Debug, Clone)]
pub enum Error {
    CallOnNonFunction,
    LabelUndefined(String),

    IllegalBreak,
    IllegalContinue,

    InvalidExpression { pos: Range<u32> },
    InvalideIterator { msg: &'static str },

    TypeError(String),
    Value(JValue),
    User(Arc<dyn Any>),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.fmt(f)
    }
}
