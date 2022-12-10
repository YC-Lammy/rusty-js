

use crate::JSValue;

pub enum Error{
    RuntimeNotAttached,
    TypeConvertionError{
        expected:&'static str,
    }
}

#[macro_export]
macro_rules! type_error {
    ($value:tt) => {
        $crate::new_type_error($value)
    };
}

pub fn new_type_error(msg:&str) -> JSValue{
    todo!()
}

impl std::fmt::Display for Error{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self{
            Self::RuntimeNotAttached => f.write_str("runtime not attached to thread"),
            Self::TypeConvertionError { expected } => f.write_fmt(format_args!("expected type {}", expected)),
        }
    }
}