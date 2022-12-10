
use std::sync::Arc;

use rusty_js_core::JSFuncContext;

use crate::{JSValue, type_error, new_type_error};

/// runtime will create a prototype according to this trait
pub trait HasPrototype where Self:Sized{
    fn constructor(args:&[JSValue]) -> Self;

    fn methods() -> &'static [(&'static str, fn(&JSFuncContext, &JSValue, &[JSValue]) -> Result<JSValue, JSValue>)]{
        return &[]
    }

    fn setters() -> &'static [(&'static str, fn(&JSFuncContext, &JSValue, &[JSValue]) -> Result<JSValue, JSValue>)]{
        return &[]
    }

    fn getters() -> &'static [(&'static str, fn(&JSFuncContext, &JSValue, &[JSValue]) -> Result<JSValue, JSValue>)]{
        return &[]
    }

    /// the __proto__ of Self.prototype
    fn prototype() -> Option<std::any::TypeId>{
        None
    }
}

pub trait Resultable{
    fn convert_result(self) -> Result<JSValue, JSValue>;
}

impl<T, E> Resultable for T where T:TryInto<JSValue, Error = E>, E:ToString{
    fn convert_result(self) -> Result<JSValue, JSValue> {
        match self.try_into(){
            Ok(v) => Ok(v),
            Err(e) => Err(new_type_error(&format!("error converting type {} to JSValue: {}", std::any::type_name::<T>(), e.to_string())))
        }
    }
}

impl<T, E, E1, E2> Resultable for Result<T, E> where T:TryInto<JSValue, Error = E1>, E:TryInto<JSValue, Error = E2>, E1:ToString, E2:ToString{
    fn convert_result(self) -> Result<JSValue, JSValue> {
        match self{
            Ok(v) => {
                return v.convert_result()
            },
            Err(e) => {
                let e = e.convert_result()?;
                return Err(e)
            }
        }
    }
}