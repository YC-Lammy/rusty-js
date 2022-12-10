
mod traits;

use std::{sync::atomic::AtomicUsize};

pub use traits::*;

mod error;
pub use error::*;

mod object;
pub use object::*;

mod runtime;
pub use runtime::*;

use rusty_js_core::{types::JValue, JSFuncContext};

#[repr(transparent)]
pub struct JSContext(JSFuncContext);

static mut JSCONTEXTS:Vec<JSContext> = Vec::new();

/// this function is exposed for macros internal use.
pub unsafe fn bind_context(ctx:&JSContext){
    JSCONTEXTS.push(JSContext(ctx.0.clone()));
}

/// this function is exposed for macros internal use.
pub unsafe fn unbind_context(){
    JSCONTEXTS.pop();
}

pub fn current_context<'a>() -> Option<&'a JSContext>{
    unsafe{JSCONTEXTS.last()}
}

/// get the new.target of the current scope
/// 
/// return None if not in new scope
pub fn new_target() -> Option<JSObject>{
    if !rusty_js_core::Runtime::is_attached(){
        return None
    }
    let rt = rusty_js_core::Runtime::current();
    let v = rt.new_target;
    if v.is_object(){
        let counter = rt.user_own_value(v);
        Some(JSObject{obj:*v.as_object().unwrap(), counter})
    } else{
        None
    }
}

// not Clone or copyable to prevent user from owning an object
pub struct JSValue{
    value:rusty_js_core::types::JValue,
    counter:*const AtomicUsize
}

impl Default for JSValue {
    fn default() -> Self {
        Self{
            value:rusty_js_core::types::JValue::UNDEFINED,
            counter:std::ptr::null()
        }
    }
}

impl Default for &JSValue{
    fn default() -> Self {
        // workaround since JSValue may not be sync or send
        thread_local! {
            static U:JSValue = JSValue::UNDEFINED;
        };
        U.with(|v|unsafe{
            std::mem::transmute_copy(&v)
        })
    }
}

impl Drop for JSValue{
    fn drop(&mut self) {
        if !self.counter.is_null(){
            unsafe{self.counter.as_ref().unwrap().fetch_sub(1, std::sync::atomic::Ordering::Relaxed)};
        }
    }
}

impl JSValue{
    pub const UNDEFINED:Self = Self{
        value:rusty_js_core::types::JValue::UNDEFINED,
        counter:std::ptr::null()
    };
}

impl TryInto<JSValue> for &JSValue{
    type Error = Error;
    fn try_into(self) -> Result<JSValue, Self::Error> {
        if self.value.is_object() || self.value.is_string(){
            if !self.counter.is_null(){
                unsafe{self.counter.as_ref().unwrap().fetch_add(1, std::sync::atomic::Ordering::Relaxed)};
                return Ok(JSValue{
                    value:self.value,
                    counter:self.counter
                })
            } else{
                if !rusty_js_core::Runtime::is_attached(){
                    return Err(Error::RuntimeNotAttached)
                }
                let rt = rusty_js_core::Runtime::current();
                let counter = rt.user_own_value(self.value);
                return Ok(JSValue { value: self.value, counter: counter })
            }
        } else{
            return Ok(JSValue { value: self.value, counter: self.counter })
        }
    }
}

impl From<()> for JSValue{
    fn from(_: ()) -> Self {
        JSValue::UNDEFINED
    }
}

impl<T> From<Option<T>> for JSValue where T:TryInto<JSValue>{
    fn from(t: Option<T>) -> Self {
        match t{
            Some(v) => {
                match v.try_into(){
                    Ok(v) => v,
                    Err(_) => JSValue::UNDEFINED
                }
            },
            None => JSValue::UNDEFINED
        }
    }
}

impl From<bool> for JSValue{
    fn from(v: bool) -> Self {
        Self{
            value:JValue::from(v),
            counter:std::ptr::null()
        }
    }
}

macro_rules! jsvalue_number {
    ($($n:ty),*) => {
        $(
            impl From<$n> for JSValue{
                fn from(v: $n) -> Self {
                    Self{
                        value:JValue::Number(v as f64),
                        counter:std::ptr::null(),
                    }
                }
            }
        )*
    };
}

jsvalue_number!(i8, i16, i32, i64, isize, u8, u16, u32, u64, usize, f32, f64);

impl TryFrom<&str> for JSValue{
    type Error = Error;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        // safty check
        if !rusty_js_core::Runtime::is_attached(){
            return Err(Error::RuntimeNotAttached)
        }
        let runtime = rusty_js_core::Runtime::current();
        let v = JValue::String(value.into());
        let counter = runtime.user_own_value(v);

        Ok(JSValue { 
            value: v, 
            counter: counter
        })
    }
}

impl TryFrom<String> for JSValue{
    type Error = Error;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        // safty check
        if !rusty_js_core::Runtime::is_attached(){
            return Err(Error::RuntimeNotAttached)
        }
        let runtime = rusty_js_core::Runtime::current();
        let v = JValue::String(value.into());
        let counter = runtime.user_own_value(v);

        Ok(JSValue { 
            value: v, 
            counter: counter
        })
    }
}

impl<T> TryFrom<Vec<T>> for JSValue where T:Into<JSValue>{
    type Error = Error;
    fn try_from(value: Vec<T>) -> Result<Self, Self::Error> {
        // safty check
        if !rusty_js_core::Runtime::is_attached(){
            return Err(Error::RuntimeNotAttached)
        }

        let ar = value.into_iter().map(|v|(rusty_js_core::bultins::prop::PropFlag::default(), Into::<JSValue>::into(v).value)).collect();
        let obj = rusty_js_core::JObject::with_array(ar);
        let v:JValue = obj.into();

        let runtime = rusty_js_core::Runtime::current();
        let counter = runtime.user_own_value(v);
        return Ok(JSValue { 
            value: v, 
            counter: counter
        });
    }
}

macro_rules! jsvalue_number_into {
    ($($n:ty),*) => {
        $(
            impl From<&JSValue> for $n{
                fn from(v: &JSValue) -> Self {
                    v.to_float() as $n
                }
            }

            impl From<&JSValue> for Option<$n>{
                fn from(v: &JSValue) -> Self {
                    if v.is_undefined(){
                        return None
                    }
                    Some(v.to_float() as $n)
                }
            }
        )*
    };
}

jsvalue_number_into!(i8, i16, i32, i64, isize, u8, u16, u32, u64, usize, f32, f64);

impl ToString for JSValue{
    fn to_string(&self) -> String {
        self.value.to_string()
    }
}

impl From<&JSValue> for Option<String>{
    fn from(v: &JSValue) -> Self {
        if v.is_undefined(){
            return None
        }
        Some(v.to_string())
    }
}

impl<'a> From<&'a JSValue> for Option<&'a str>{
    fn from(v: &'a JSValue) -> Self {
        v.as_string()
    }
}

impl From<&JSValue> for bool{
    fn from(v: &JSValue) -> Self {
        v.to_bool()
    }
}

impl From<&JSValue> for Option<bool>{
    fn from(v: &JSValue) -> Self {
        if v.is_undefined(){
            return None
        }
        Some(v.to_bool())
    }
}

impl JSValue{
    pub fn to_owned(&self, ctx:&JSContext) -> JSValue{
        
        if !self.counter.is_null(){
            unsafe{self.counter.as_ref().unwrap().fetch_add(1, std::sync::atomic::Ordering::Relaxed)};
            return JSValue{value:self.value, counter:self.counter}
        }

        if self.is_object() || self.is_string(){
            let counter = ctx.0.runtime.user_own_value(self.value);
            JSValue { value: self.value, counter}
        } else{
            JSValue { value: self.value, counter: std::ptr::null() }
        }
    }

    pub fn is_undefined(&self) -> bool{
        self.value.is_undefined()
    }

    pub fn is_bool(&self) -> bool{
        self.value.is_bool()
    }

    pub fn is_null(&self) -> bool{
        self.value.is_null()
    }

    pub fn is_number(&self) -> bool{
        self.value.is_number()
    }

    pub fn is_bigint(&self) -> bool{
        self.value.is_bigint()
    }

    pub fn is_string(&self) -> bool{
        self.value.is_string()
    }

    pub fn is_symbol(&self) -> bool{
        self.value.is_symbol()
    }

    pub fn is_object(&self) -> bool{
        self.value.is_object()
    }
    
    pub fn to_float(&self) -> f64{
        self.value.to_number()
    }

    pub fn to_bool(&self) -> bool{
        self.value.to_bool()
    }

    // prevent the user from owning the object
    /// borrows an object from the value if any
    pub fn as_object<'a>(&'a self) -> Option<&'a JSBorrowObject>{
        let obj = self.value.as_object()?;
        Some(unsafe{std::mem::transmute_copy(&obj)})
    }

    /// return an owned object if the object has already been owned
    /// if the object is not owned by the user, this will return None
    /// 
    /// returning None does not mean JSValue is not an object.
    pub fn as_owned_object(&self) -> Option<JSObject>{
        if let Some(o) = self.value.as_object(){
            if !self.counter.is_null(){
                let obj = JSObject { obj: *o, counter: self.counter };
                let o = obj.clone();
                std::mem::forget(obj);
                return Some(o);
            }
        }
        None
    }

    pub fn as_custom_object<T:CustomObject+'static>(&self) -> Option<&T>{
        if let Some(o) = self.as_object(){
            o.as_custom_object()
        } else{
            None
        }
    }

    pub fn as_mut_custom_object<T:CustomObject+'static>(&self) -> Option<&mut T>{
        if let Some(o) = self.as_object(){
            o.as_custom_object_mut()
        } else{
            None
        }
    }

    /// strings in Javascript are not mutable,
    /// user must own the String in order mutate
    pub fn as_string<'a>(&'a self) -> Option<&'a str>{
        if let Some(v) = self.value.as_string(){
            Some(v.as_str())
        } else{
            None
        }  
    }

    /// get the number of user owned reference count.
    /// 
    /// if the count is zero and value is an object,
    /// the object is not owned by the user
    pub unsafe fn user_reference_count(&self) -> usize{
        if self.counter.is_null(){
            return 0
        }

        self.counter.as_ref().unwrap().load(std::sync::atomic::Ordering::Relaxed)
    }
}