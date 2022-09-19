use std::{ops::Range, any::Any, sync::Arc};

use crate::types::JValue;


#[derive(Debug, Clone)]
pub enum Error{
    CallOnNonFunction,
    LabelUndefined(String),

    IllegalBreak,
    IllegalContinue,

    InvalidExpression{
        pos:Range<u32>
    },
    InvalideIterator{
        msg:&'static str
    },
    

    TypeError(String),
    Value(JValue),
    User(Arc<dyn Any>),
}