use std::{
    borrow::Cow,
    fmt::Write,
    ops::{self, Deref},
};

use crate::{
    runtime::{GcFlag, Runtime},
    value::JValue,
};

#[derive(Debug, Clone, Copy)]
pub struct JSString(pub(crate) *mut u8);

impl JSString {
    pub fn len(&self) -> usize {
        if self.0.is_null() {
            return 0;
        }
        unsafe { u32::from_ne_bytes(*(self.0 as *mut [u8; 4])) as usize }
    }

    pub fn as_str(&self) -> &str {
        if self.0.is_null() {
            return "";
        }
        unsafe {
            let slice = std::slice::from_raw_parts(self.0.add(4), self.len());
            std::str::from_utf8_unchecked(slice)
        }
    }

    pub fn trace(&self) {}
}

impl std::fmt::Display for JSString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl ops::Add for JSString {
    type Output = JSString;
    fn add(self, rhs: Self) -> Self::Output {
        let rt = Runtime::current();
        let s = self.to_string() + rhs.as_str();
        rt.allocate_string(&s)
    }
}

impl ops::Add<&str> for JSString {
    type Output = JSString;
    fn add(self, rhs: &str) -> Self::Output {
        self + JSString::from(rhs)
    }
}

impl ops::Add<JSString> for String {
    type Output = String;
    fn add(self, rhs: JSString) -> Self::Output {
        self + rhs.as_ref()
    }
}

impl From<String> for JSString {
    fn from(s: String) -> Self {
        s.as_str().into()
    }
}

impl From<&str> for JSString {
    fn from(s: &str) -> Self {
        let rt = Runtime::current();
        rt.allocate_string(s)
    }
}

impl ops::Deref for JSString {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl AsRef<str> for JSString {
    fn as_ref(&self) -> &str {
        return self.deref();
    }
}

pub struct Template {
    pub strings: Vec<String>,
    //pub total_length:usize,
    pub tagged: bool,
}

impl Template {
    pub fn create(&self, args: &[Cow<str>]) -> JValue {
        let mut s = String::new();
        let mut iter = args.iter();
        for i in &self.strings {
            s += &i;
            if let Some(v) = iter.next() {
                s += v;
            }
        }
        JValue::create_string(s.into())
    }
}
