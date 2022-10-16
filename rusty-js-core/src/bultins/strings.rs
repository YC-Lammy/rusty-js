use std::{
    borrow::Cow,
    ops::{self, Deref},
};

use crate::{
    runtime::{GcFlag, Runtime},
    types::JValue,
};


#[derive(Debug, Clone, Copy, PartialEq)]
pub enum JStringType {
    Combind,
    String,
    Static,
}

// GcFlag must be put first
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct JStringHeader {
    flag: GcFlag,
    size: usize,
    type_: JStringType,
}


/// String::Combind:
/// 
///     JStringHeader | [*const JStringHeader;2]
/// 
/// String::String:
/// 
///     JStringHeader | [u8]...
/// 
/// String::Static:
/// 
///     JStringHeader | *const u8
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct JString {
    value: *const JStringHeader,
}

impl ops::Add for JString {
    type Output = JString;
    fn add(self, rhs: Self) -> Self::Output {
        if self.value.is_null() {
            return rhs;
        } else if rhs.value.is_null() {
            return self;
        };

        unsafe {
            let ptr = Self::allocate(2 * std::mem::size_of::<*const JStringHeader>());
            let a = ptr.add(1) as *mut *const JStringHeader;
            *a = self.value;
            *(a.add(1)) = rhs.value;
            *ptr = JStringHeader {
                flag: GcFlag::Used,
                size: 2,
                type_: JStringType::Combind,
            };

            Self { value: ptr }
        }
    }
}

impl ops::Add<&str> for JString {
    type Output = JString;
    fn add(self, rhs: &str) -> Self::Output {
        self + JString::from(rhs)
    }
}

impl ops::Add<JString> for String {
    type Output = String;
    fn add(self, rhs: JString) -> Self::Output {
        self + rhs.as_ref()
    }
}

impl From<String> for JString {
    fn from(s: String) -> Self {
        s.as_str().into()
    }
}

impl From<&str> for JString {
    fn from(s: &str) -> Self {
        if s.len() == 0 {
            return Self {
                value: 0 as *const _,
            };
        }
        let ptr = Self::allocate(s.len());
        unsafe {
            *ptr = JStringHeader {
                flag: GcFlag::Used,
                size: s.len(),
                type_: JStringType::String,
            };
            let p = ptr.add(1) as *mut u8;
            std::ptr::copy_nonoverlapping(s.as_ptr(), p, s.len());
        };
        return JString { value: ptr };
    }
}

impl ops::Deref for JString {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        if self.value.is_null() {
            return "";
        }
        self.combind();
        unsafe {
            let header = *self.value;

            let ptr = match header.type_ {
                JStringType::String => self.value.add(1) as *const u8,
                JStringType::Static => {
                    let p = self.value.add(1) as *const *const u8;
                    *p
                }
                JStringType::Combind => unreachable!(),
            };
            return std::str::from_utf8(std::slice::from_raw_parts(ptr, header.size)).unwrap();
        }
    }
}

impl AsRef<str> for JString {
    fn as_ref(&self) -> &str {
        return self.deref();
    }
}

impl JString {

    pub fn from_static(s: &'static str) -> JString {
        if s.len() == 0 {
            return JString {
                value: std::ptr::null(),
            };
        }

        let ptr = Self::allocate(std::mem::size_of::<*const u8>());
        unsafe {
            *ptr = JStringHeader {
                flag: GcFlag::Used,
                size: s.len(),
                type_: JStringType::Static,
            };
            let p = ptr.add(1) as *mut *const u8;
            *p = s.as_ptr();
        };
        JString { value: ptr }
    }

    pub fn as_str(&self) -> &str{
        self
    }

    fn combind(&self) {
        if self.value.is_null() {
            return;
        }

        let header = unsafe { *self.value };

        match header.type_ {
            JStringType::Combind => unsafe {
                let ptr = self.value.add(1) as *const *const JStringHeader;
                let ptr = std::slice::from_raw_parts(ptr, header.size);

                let mut ptrs = Vec::new();
                let mut len = 0;

                for v in ptr {
                    let v = JString { value: *v };

                    if v.value.is_null() {
                        continue;
                    }

                    v.combind();
                    let header = *v.value;
                    match header.type_ {
                        JStringType::String => {
                            // string stores bytes after header
                            let ptr = v.value.add(1) as *const u8;
                            ptrs.push((header.size, ptr));
                            len += header.size
                        }
                        JStringType::Static => {
                            // static stores a pointer after header
                            let ptr = v.value.add(1) as *const *const u8;
                            ptrs.push((header.size, *ptr));
                            len += header.size
                        }
                        JStringType::Combind => unreachable!(),
                    };
                }

                if len == 0 {
                    *(self as *const Self as *mut Self) = Self {
                        value: std::ptr::null(),
                    }
                }

                let new = Self::allocate(len);
                let ptr = new.add(1) as *mut u8;
                let mut offset = 0;

                for (len, p) in ptrs {
                    std::ptr::copy_nonoverlapping(p, ptr.add(offset), len);
                    offset += len;
                }
                *new = JStringHeader {
                    flag: GcFlag::Used,
                    size: len,
                    type_: JStringType::String,
                };

                *(self as *const Self as *mut Self) = Self { value: new }
            },
            JStringType::String => {}
            JStringType::Static => {}
        }
    }

    pub fn trace(&self) {
        self.combind();
        unsafe { (*(self.value as *mut JStringHeader)).flag = GcFlag::Used };
    }

    /// allocate size + size_of::<JStringHeader>()
    fn allocate(size: usize) -> *mut JStringHeader {
        let runtime = Runtime::current();
        runtime.allocate_string(size) as *mut _
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
        JValue::String(s.into())
    }
}
