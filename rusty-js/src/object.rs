
use std::ops::Deref;
use std::ops::DerefMut;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

use rusty_js_core::JObject;
use rusty_js_core::JSFuncContext;

use crate::JSContext;
use crate::JSValue;
use crate::new_type_error;

pub trait CustomObject{
    fn get(&mut self, name:&str) -> Option<&JSValue>;
    fn set(&mut self, name:&str, value:&JSValue) -> bool;
    fn own_properties(&self) -> &[&str];

    #[allow(unused_variables)]
    fn call(&mut self, this:&JSValue, args:&[JSValue]) -> Result<JSValue, JSValue>{
        return Err(new_type_error("call on non callable object"))
    }
}

pub(crate) struct CustomObjectWrapper<T>(pub(crate) T) where T:CustomObject;

impl<T> rusty_js_core::CustomObject for CustomObjectWrapper<T> where T:CustomObject+'static{
    fn get(&mut self, name:&str) -> Option<rusty_js_core::JValue> {
        self.0.get(name).and_then(|v|Some(v.value))
    }
    fn set(&mut self, name:&str, value:&rusty_js_core::JValue) -> bool {
        let value = JSValue{
            value:*value,
            counter:std::ptr::null()
        };
        self.0.set(name, &value)
    }
    fn own_properties(&self) -> &[&str] {
        self.0.own_properties()
    }
    fn call(&mut self, this:&rusty_js_core::JValue, args:&[rusty_js_core::JValue]) -> Result<rusty_js_core::JValue, rusty_js_core::JValue> {
        let this = JSValue { value: *this, counter: std::ptr::null() };
        let args = args.iter().map(|v|JSValue{value:*v, counter:std::ptr::null()}).collect::<Vec<JSValue>>();
        let re = self.0.call(&this, &args);
        match re{
            Ok(v) => Ok(v.value),
            Err(e) => Err(e.value)
        }
    }
}

pub struct JSObject{
    pub(crate) obj:JObject,
    pub(crate) counter:*const AtomicUsize
}

#[repr(transparent)]
pub struct JSBorrowObject(pub(crate) JObject);

impl Clone for JSObject{
    fn clone(&self) -> Self {
        if self.counter.is_null(){

        }
        unsafe{self.counter.as_ref().unwrap().fetch_add(1, Ordering::Relaxed)};
        Self { obj: self.obj, counter: self.counter }
    }
}

impl AsRef<JSBorrowObject> for JSObject{
    fn as_ref(&self) -> &JSBorrowObject {
        unsafe{std::mem::transmute_copy(&&self.obj)}
    }
}

impl AsMut<JSBorrowObject> for JSObject{
    fn as_mut(&mut self) -> &mut JSBorrowObject {
        unsafe{std::mem::transmute_copy(&&self.obj)}
    }
}

impl Deref for JSObject{
    type Target = JSBorrowObject;
    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl DerefMut for JSObject{
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut()
    }
}

impl JSBorrowObject{
    pub fn get(&self, ctx:&JSContext, key:&str) -> Option<JSValue>{
        let re = self.0.get_property(key, ctx.0.stack);
        match re{
            None => None,
            Some(v) => {
                if v.is_object() || v.is_string(){
                    let counter = ctx.0.runtime.user_own_value(v);
                    Some(JSValue { value: v, counter})
                } else{
                    Some(JSValue { value: v, counter: std::ptr::null() })
                }
            }
        }
    }

    pub fn set(&self, ctx:&JSContext, name:&str, value:&JSValue){
        self.0.set_property(name, value.value, ctx.0.stack);
    }

    pub fn as_custom_object<'a, T:CustomObject + 'static>(&'a self) -> Option<&'a T>{
        match self.0.as_custom_object(){
            None => None,
            Some(o) => {
                let rep = o.as_ref().downcast_ref::<CustomObjectWrapper<T>>();
                if let Some(v) = rep{
                    Some(&v.0)
                } else{
                    None
                }
            }
        }
    }

    pub fn as_custom_object_mut<'a, T:CustomObject + 'static>(&'a self) -> Option<&'a mut T>{
        let r = self.as_custom_object::<T>();
        if let Some(r) = r{
            Some(unsafe{std::mem::transmute_copy(&r)})
        } else{None}
    }
}