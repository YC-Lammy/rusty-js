use crate::bultins::function::JSFuncContext;
use crate::bultins::object::JObject;
use crate::bultins::promise::Promise;
use crate::bultins::strings::JString;
use crate::bultins::symbol::JSymbol;
use crate::bytecodes::TempAllocValue;
use crate::error::Error;
use crate::fast_iter::FastIterator;
use crate::runtime::{AsyncResult, Runtime};

pub struct JSResult(pub Result<JValue, JValue>);

impl From<(JValue, bool)> for JSResult {
    fn from(v: (JValue, bool)) -> Self {
        if v.1 {
            JSResult(Err(v.0))
        } else {
            JSResult(Ok(v.0))
        }
    }
}

impl std::ops::Deref for JSResult {
    type Target = Result<JValue, JValue>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for JSResult {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct JValue {
    pub(crate) value: JValueUnion,
    pub(crate) type_pointer: &'static JTypeVtable,
}

unsafe impl Sync for JValue {}
unsafe impl Send for JValue {}

impl std::hash::Hash for JValue {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        if self.is_string(){
            unsafe{self.value.string.as_bytes().hash(state)}
        } else{
            state.write_usize(unsafe { std::mem::transmute(self.value) });
            state.write_usize(self.type_pointer as *const _ as usize); 
        }
        
    }
}

impl std::fmt::Debug for JValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        unsafe {
            if self.is_bigint() {
                f.write_fmt(format_args!("BigInt({})", self.value.bigint))
            } else if self.is_bool() {
                f.write_fmt(format_args!("Boolean({})", self.is_true()))
            } else if self.is_null() {
                f.write_str("null")
            } else if self.is_number() {
                f.write_fmt(format_args!("Number({})", self.value.number))
            } else if self.is_object() {
                f.write_str("[object Object]")
            } else if self.is_string() {
                f.write_fmt(format_args!("String({})", self.value.string.as_ref()))
            } else if self.is_symbol() {
                f.write_str("Symbol")
            } else {
                f.write_str("undefined")
            }
        }
    }
}

#[cfg(target_pointer_width = "64")]
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) union JValueUnion {
    pub number: f64,
    pub bigint: i64,
    pub string: JString,
    pub symbol: JSymbol,
    pub object: JObject,

    pub null: u64,
    pub undefined: u64,
    pub real_undefined: u64,
    pub true_: u64,
    pub false_: u64,
}

#[cfg(target_pointer_width = "32")]
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) union JValueUnion {
    pub number: f32,
    pub bigint: i32,
    pub string: JString,
    pub symbol: JSymbol,
    pub object: JObject,

    pub null: u32,
    pub undefined: u32,
    pub real_undefined: u32,
    pub true_: u32,
    pub false_: u32,
}

/*
pub trait JSValuable{
    unsafe fn add(value:JValueUnion, rhs:JValue) -> (JValue, bool);
    unsafe fn sub(value:JValueUnion, rhs:JValue) -> (JValue, bool);
    unsafe fn mul(value:JValueUnion, rhs:JValue) -> (JValue, bool);
    unsafe fn div(value:JValueUnion, rhs:JValue) -> (JValue, bool);
    unsafe fn rem(value:JValueUnion, rhs:JValue) -> (JValue, bool);

    unsafe fn lshift(value:JValueUnion, rhs:JValue) -> (JValue, bool);
    unsafe fn rshift(value:JValueUnion, rhs:JValue) -> (JValue, bool);

    unsafe fn gt(value:JValueUnion, rhs:JValue) -> (JValue, bool);
    unsafe fn gteq(value:JValueUnion, rhs:JValue) -> (JValue, bool);
    unsafe fn lt(value:JValueUnion, rhs:JValue) -> (JValue, bool);
    unsafe fn lteq(value:JValueUnion, rhs:JValue) -> (JValue, bool);

    unsafe fn instanceOf(value:JValueUnion, rhs:JValue) -> (JValue, bool);
    unsafe fn In(value:JValueUnion, rhs:JValue) -> (JValue, bool);

    unsafe fn set(obj:JValueUnion, key:JValue, value:JValue) -> (JValue, bool);
    unsafe fn set_static(obj:JValueUnion, key:u32, value:JValue) -> (JValue, bool);
    unsafe fn get(obj:JValueUnion, key:JValue) -> (JValue, bool);
    unsafe fn get_static(obj:JValueUnion, key:u32) -> (JValue, bool);
    unsafe fn remove_key_static(obj:JValueUnion, key:u32);
}
*/

#[derive(Debug)]
#[repr(C)]
pub struct JTypeVtable {
    add: unsafe fn(JValueUnion, JValue) -> (JValue, bool),
    sub: unsafe fn(JValueUnion, JValue) -> (JValue, bool),
    mul: unsafe fn(JValueUnion, JValue) -> (JValue, bool),
    div: unsafe fn(JValueUnion, JValue) -> (JValue, bool),
    rem: unsafe fn(JValueUnion, JValue) -> (JValue, bool),
    exp: unsafe fn(JValueUnion, JValue) -> (JValue, bool),
    eqeq: unsafe fn(JValueUnion, JValue) -> (JValue, bool),
    noteq: unsafe fn(JValueUnion, JValue) -> (JValue, bool),

    set: unsafe fn(JValueUnion, JValue, JValue, *mut JValue) -> (JValue, bool),
    set_static: unsafe fn(JValueUnion, u32, JValue, *mut JValue) -> (JValue, bool),
    get: unsafe fn(JValueUnion, JValue, *mut JValue) -> (JValue, bool),
    get_static: unsafe fn(JValueUnion, u32, *mut JValue) -> (JValue, bool),
    remove_key_static: unsafe fn(JValueUnion, u32),

    gt: unsafe fn(JValueUnion, JValue) -> (JValue, bool),
    gteq: unsafe fn(JValueUnion, JValue) -> (JValue, bool),
    lt: unsafe fn(JValueUnion, JValue) -> (JValue, bool),
    lteq: unsafe fn(JValueUnion, JValue) -> (JValue, bool),

    instanceOf: unsafe fn(JValueUnion, JValue) -> (JValue, bool),
    /// fn (obj, field) -> (result, error)
    In: unsafe fn(JValueUnion, JValue) -> (JValue, bool),
}

impl JTypeVtable {
    const T: Self = NULL_TYPE_POINTER;

    pub const TRUE: Self = TRUE_TYPE_POINTER;
    pub const FALSE: Self = FALSE_TYPE_POINTER;
    pub const NULL: Self = NULL_TYPE_POINTER;
    pub const UNDEFINED: Self = UNDEFINED_TYPE_POINTER;
    pub const BIGINT: Self = BIGINT_TYPE_POINTER;
    pub const NUMBER: Self = NUMBER_TYPE_POINTER;
    pub const SYMBOL: Self = SYMBOL_TYPE_POINTER;

    pub fn offset_add() -> i32 {
        (&Self::T.add as *const _ as usize - &Self::T as *const _ as usize) as i32
    }
    pub fn offset_sub() -> i32 {
        (&Self::T.sub as *const _ as usize - &Self::T as *const _ as usize) as i32
    }
    pub fn offset_mul() -> i32 {
        (&Self::T.mul as *const _ as usize - &Self::T as *const _ as usize) as i32
    }
    pub fn offset_div() -> i32 {
        (&Self::T.div as *const _ as usize - &Self::T as *const _ as usize) as i32
    }
    pub fn offset_rem() -> i32 {
        (&Self::T.rem as *const _ as usize - &Self::T as *const _ as usize) as i32
    }

    pub fn offset_exp() -> i32 {
        (&Self::T.exp as *const _ as usize - &Self::T as *const _ as usize) as i32
    }

    pub fn offset_eqeq() -> i32 {
        (&Self::T.eqeq as *const _ as usize - &Self::T as *const _ as usize) as i32
    }

    pub fn offset_noteq() -> i32 {
        (&Self::T.noteq as *const _ as usize - &Self::T as *const _ as usize) as i32
    }

    pub fn offset_set() -> i32 {
        (&Self::T.set as *const _ as usize - &Self::T as *const _ as usize) as i32
    }

    pub fn offset_set_static() -> i32 {
        (&Self::T.set_static as *const _ as usize - &Self::T as *const _ as usize) as i32
    }

    pub fn offset_get() -> i32 {
        (&Self::T.get as *const _ as usize - &Self::T as *const _ as usize) as i32
    }

    pub fn offset_get_static() -> i32 {
        (&Self::T.get_static as *const _ as usize - &Self::T as *const _ as usize) as i32
    }

    pub fn offset_remove_key_static() -> i32 {
        (&Self::T.remove_key_static as *const _ as usize - &Self::T as *const _ as usize) as i32
    }

    pub fn offset_gt() -> i32 {
        (&Self::T.gt as *const _ as usize - &Self::T as *const _ as usize) as i32
    }

    pub fn offset_gteq() -> i32 {
        (&Self::T.gteq as *const _ as usize - &Self::T as *const _ as usize) as i32
    }

    pub fn offset_lt() -> i32 {
        (&Self::T.lt as *const _ as usize - &Self::T as *const _ as usize) as i32
    }

    pub fn offset_lteq() -> i32 {
        (&Self::T.lteq as *const _ as usize - &Self::T as *const _ as usize) as i32
    }

    pub fn offset_instanceOf() -> i32 {
        (&Self::T.instanceOf as *const _ as usize - &Self::T as *const _ as usize) as i32
    }

    pub fn offset_In() -> i32 {
        (&Self::T.In as *const _ as usize - &Self::T as *const _ as usize) as i32
    }
}

impl Default for JTypeVtable {
    fn default() -> Self {
        unsafe {
            Self {
                add: std::mem::transmute::<usize, _>(0),
                sub: std::mem::transmute::<usize, _>(0),
                mul: std::mem::transmute::<usize, _>(0),
                div: std::mem::transmute::<usize, _>(0),
                rem: std::mem::transmute::<usize, _>(0),
                exp: std::mem::transmute::<usize, _>(0),
                eqeq: std::mem::transmute::<usize, _>(0),
                noteq: std::mem::transmute::<usize, _>(0),
                set: std::mem::transmute::<usize, _>(0),
                set_static: std::mem::transmute::<usize, _>(0),
                get: std::mem::transmute::<usize, _>(0),
                get_static: std::mem::transmute::<usize, _>(0),
                remove_key_static: std::mem::transmute::<usize, _>(0),
                gt: std::mem::transmute::<usize, _>(0),
                gteq: std::mem::transmute::<usize, _>(0),
                lt: std::mem::transmute::<usize, _>(0),
                lteq: std::mem::transmute::<usize, _>(0),
                instanceOf: std::mem::transmute::<usize, _>(0),
                In: std::mem::transmute::<usize, _>(0),
            }
        }
    }
}

const NULL_TYPE_POINTER: JTypeVtable = JTypeVtable {
    add: null::add,
    sub: null::sub,
    mul: null::mul,
    div: null::div,
    rem: null::rem,
    exp: null::exp,
    eqeq: null::eqeq,
    noteq: null::noteq,
    gt: null::gt,
    gteq: null::gteq,
    lt: null::lt,
    lteq: null::lteq,

    get: notObject::get,
    get_static: notObject::get_static,
    set: notObject::set,
    set_static: notObject::set_static,
    remove_key_static: notObject::remove_key_static,
    instanceOf: notObject::instanceOf,
    In: notObject::In,
};

const UNDEFINED_TYPE_POINTER: JTypeVtable = JTypeVtable {
    add: undefined::add,
    sub: undefined::sub,
    mul: undefined::mul,
    div: undefined::div,
    rem: undefined::rem,
    exp: undefined::exp,
    eqeq: undefined::eqeq,
    noteq: undefined::noteq,
    gt: undefined::gt,
    gteq: undefined::gteq,
    lt: undefined::lt,
    lteq: undefined::lteq,

    get: notObject::get,
    get_static: notObject::get_static,
    set: notObject::set,
    set_static: notObject::set_static,
    remove_key_static: notObject::remove_key_static,
    instanceOf: notObject::instanceOf,
    In: notObject::In,
};

const REAL_UNDEFINED_TYPE_POINTER: JTypeVtable = JTypeVtable {
    ..UNDEFINED_TYPE_POINTER
};

const TRUE_TYPE_POINTER: JTypeVtable = JTypeVtable {
    add: true_::add,
    sub: true_::sub,
    mul: true_::mul,
    div: true_::div,
    rem: true_::rem,
    exp: true_::exp,
    eqeq: true_::eqeq,
    noteq: true_::noteq,
    gt: true_::gt,
    gteq: true_::gteq,
    lt: true_::lt,
    lteq: true_::lteq,

    get: notObject::get,
    get_static: notObject::get_static,
    set: notObject::set,
    set_static: notObject::set_static,
    remove_key_static: notObject::remove_key_static,
    instanceOf: notObject::instanceOf,
    In: notObject::In,
};

const FALSE_TYPE_POINTER: JTypeVtable = JTypeVtable {
    add: false_::add,
    sub: false_::sub,
    mul: false_::mul,
    div: false_::div,
    rem: false_::rem,
    exp: false_::exp,
    eqeq: false_::eqeq,
    noteq: false_::noteq,
    gt: false_::gt,
    gteq: false_::gteq,
    lt: false_::lt,
    lteq: false_::lteq,

    get: notObject::get,
    get_static: notObject::get_static,
    set: notObject::set,
    set_static: notObject::set_static,
    remove_key_static: notObject::remove_key_static,
    instanceOf: notObject::instanceOf,
    In: notObject::In,
};

const NUMBER_TYPE_POINTER: JTypeVtable = JTypeVtable {
    add: number::add,
    sub: number::sub,
    mul: number::mul,
    div: number::div,
    rem: number::rem,
    exp: number::exp,
    eqeq: number::eqeq,
    noteq: number::noteq,
    gt: number::gt,
    gteq: number::gteq,
    lt: number::lt,
    lteq: number::lteq,

    get: notObject::get,
    get_static: notObject::get_static,
    set: notObject::set,
    set_static: notObject::set_static,
    remove_key_static: notObject::remove_key_static,
    instanceOf: notObject::instanceOf,
    In: notObject::In,
};

const BIGINT_TYPE_POINTER: JTypeVtable = JTypeVtable {
    add: bigint::add,
    sub: bigint::sub,
    mul: bigint::mul,
    div: bigint::div,
    rem: bigint::rem,
    exp: bigint::exp,
    eqeq: bigint::eqeq,
    noteq: bigint::noteq,
    gt: bigint::gt,
    gteq: bigint::gteq,
    lt: bigint::lt,
    lteq: bigint::lteq,

    get: notObject::get,
    get_static: notObject::get_static,
    set: notObject::set,
    set_static: notObject::set_static,
    remove_key_static: notObject::remove_key_static,
    instanceOf: notObject::instanceOf,
    In: notObject::In,
};

const STRING_TYPE_POINTER: JTypeVtable = JTypeVtable {
    add: string::add,
    sub: string::sub,
    mul: string::mul,
    div: string::div,
    rem: string::rem,
    exp: string::exp,
    eqeq: string::eqeq,
    noteq: string::noteq,
    gt: string::gt,
    gteq: string::gteq,
    lt: string::lt,
    lteq: string::lteq,

    get: string::get,
    get_static: string::get_static,
    set: string::set,
    set_static: string::set_static,
    remove_key_static: string::remove_key_static,

    instanceOf: notObject::instanceOf,
    In: notObject::In,
};

const SYMBOL_TYPE_POINTER: JTypeVtable = JTypeVtable {
    add: symbol::throw,
    sub: symbol::throw,
    mul: symbol::throw,
    div: symbol::throw,
    rem: symbol::throw,
    eqeq: symbol::throw,
    noteq: symbol::throw,
    exp: symbol::throw,
    gt: symbol::throw,
    gteq: symbol::throw,
    lt: symbol::throw,
    lteq: symbol::throw,

    get: notObject::get,
    get_static: notObject::get_static,
    set: notObject::set,
    set_static: notObject::set_static,
    remove_key_static: notObject::remove_key_static,
    instanceOf: notObject::instanceOf,
    In: notObject::In,
};

const OBJECT_TYPE_POINTER: JTypeVtable = JTypeVtable {
    add: object::add,
    sub: object::sub,
    mul: object::mul,
    div: object::div,
    rem: object::rem,
    exp: object::exp,
    eqeq: object::eqeq,
    noteq: object::noteq,
    get: object::get,
    get_static: object::get_static,
    set: object::set,
    set_static: object::set_static,
    remove_key_static: object::remove_key_static,

    gt: object::False,
    gteq: object::False,
    lt: object::False,
    lteq: object::False,

    instanceOf: object::instanceOf,
    In: object::In,
};

#[allow(non_snake_case)]
impl JValue {
    pub const SIZE: usize = std::mem::size_of::<Self>();
    pub const VALUE_SIZE: usize = std::mem::size_of::<JValueUnion>();
    pub const VTABLE_SIZE: usize = std::mem::size_of::<*const JTypeVtable>();

    pub const NULL: JValue = JValue {
        value: JValueUnion { null: 0 },
        type_pointer: &NULL_TYPE_POINTER,
    };

    pub const TRUE: JValue = JValue {
        value: JValueUnion { true_: 1 },
        type_pointer: &TRUE_TYPE_POINTER,
    };

    pub const FALSE: JValue = JValue {
        value: JValueUnion { false_: 0 },
        type_pointer: &FALSE_TYPE_POINTER,
    };

    pub const UNDEFINED: JValue = JValue {
        value: JValueUnion { undefined: 0 },
        type_pointer: &UNDEFINED_TYPE_POINTER,
    };

    /*
    pub const REAL_UNDEFINED:JValue = JValue{
        value:JValueUnion { real_undefined: 0 },
        type_pointer:&REAL_UNDEFINED_TYPE_POINTER
    };
    */

    pub fn is_null(&self) -> bool {
        return self.type_pointer as *const _ == &NULL_TYPE_POINTER;
    }

    pub fn is_undefined(&self) -> bool {
        return (self.type_pointer as *const _ == &UNDEFINED_TYPE_POINTER);
    }

    //pub fn is_real_undefined(&self) -> bool{
    //return self.type_pointer as *const _ == &REAL_UNDEFINED_TYPE_POINTER
    //}

    pub fn is_bool(&self) -> bool {
        return self.type_pointer as *const _ == &TRUE_TYPE_POINTER
            || self.type_pointer as *const _ == &FALSE_TYPE_POINTER;
    }

    pub fn is_true(&self) -> bool {
        return self.type_pointer as *const _ == &TRUE_TYPE_POINTER;
    }

    pub fn is_false(&self) -> bool {
        return self.type_pointer as *const _ == &FALSE_TYPE_POINTER;
    }

    pub fn is_number(&self) -> bool {
        return self.type_pointer as *const _ == &NUMBER_TYPE_POINTER;
    }

    pub fn is_bigint(&self) -> bool {
        return self.type_pointer as *const _ == &BIGINT_TYPE_POINTER;
    }

    pub fn is_string(&self) -> bool {
        return self.type_pointer as *const _ == &STRING_TYPE_POINTER;
    }

    pub fn is_symbol(&self) -> bool {
        return self.type_pointer as *const _ == &SYMBOL_TYPE_POINTER;
    }

    pub fn is_object(&self) -> bool {
        return self.type_pointer as *const JTypeVtable == &OBJECT_TYPE_POINTER;
    }

    pub fn Number(n: f64) -> JValue {
        return JValue {
            value: JValueUnion { number: n },
            type_pointer: &NUMBER_TYPE_POINTER,
        };
    }

    pub fn BigInt(n: i64) -> JValue {
        return JValue {
            value: JValueUnion { bigint: n },
            type_pointer: &BIGINT_TYPE_POINTER,
        };
    }

    pub fn String(s: JString) -> JValue {
        return JValue {
            value: JValueUnion { string: s },
            type_pointer: &STRING_TYPE_POINTER,
        };
    }

    pub fn Object(o: JObject) -> JValue {
        return JValue {
            value: JValueUnion { object: o },
            type_pointer: &OBJECT_TYPE_POINTER,
        };
    }

    pub fn Error(e: Error) -> JValue {
        let obj = JObject::with_error(e);
        return JValue {
            value: JValueUnion { object: obj },
            type_pointer: &OBJECT_TYPE_POINTER,
        };
    }

    pub fn as_number_uncheck(&self) -> f64 {
        unsafe { self.value.number }
    }

    pub fn as_promise(&self) -> Option<&crate::bultins::promise::Promise> {
        if self.is_object() {
            unsafe { self.value.object.inner.wrapped_value.promise() }
        } else {
            None
        }
    }

    pub fn to_bool(self) -> bool {
        if unsafe { std::mem::transmute::<_, u64>(self.value) } == 0 {
            false
        } else {
            true
        }
    }

    pub fn to_number(self) -> f64 {
        if self.is_bigint() {
            unsafe { self.value.bigint as f64 }
        } else if self.is_true() {
            1.0
        } else if self.is_false() {
            0.0
        } else if self.is_null() {
            0.0
        } else if self.is_number() {
            unsafe { self.value.number }
        } else if self.is_object() {
            if let Some(f) = unsafe { self.value.object.inner.wrapped_value.number() } {
                f
            } else if let Some(s) = unsafe { self.value.object.inner.wrapped_value.string() } {
                if let Ok(v) = s.parse::<f64>() {
                    v
                } else {
                    0.0
                }
            } else {
                0.0
            }
        } else if self.is_string() {
            if unsafe { self.value.string.len() } == 0 {
                return 0.0;
            }
            if let Ok(v) = unsafe { self.value.string.parse::<f64>() } {
                v
            } else {
                0.0
            }
        } else if self.is_symbol() {
            0.0
        } else {
            0.0
        }
    }

    pub fn to_i32(self) -> i32 {
        self.to_number() as i32
    }

    pub fn bitnot(self) -> Self {
        JValue::Number((!self.to_i32()) as f64)
    }

    pub fn zerofillRshift(self, rhs: Self) -> Self {
        JValue::Number(((self.to_i32() as u32) << (rhs.to_i32() as u32)) as f64)
    }

    /// * wait for a value, block until future is fulfilled
    /// * return immediately if value is not future
    pub(crate) fn wait(self) -> (Self, bool) {
        if let Some(p) = self.as_promise() {
            match p {
                Promise::Fulfilled(f) => (*f, false),
                Promise::Rejected(v) => (*v, true),
                Promise::Pending { id } => {
                    let runtime = Runtime::current();
                    loop {
                        let re = runtime.to_mut().poll_async(*id, JValue::UNDEFINED);
                        match re {
                            AsyncResult::Err(e) => return (e, true),
                            AsyncResult::Return(v) => return (v, false),
                            AsyncResult::Yield(_) => {}
                        }
                    }
                }
            }
        } else {
            return (self, false);
        }
    }

    pub unsafe fn private_in(self, name: u32) -> JValue {
        if self.is_object() {
            self.value.object.has_owned_property_static(name).into()
        } else {
            JValue::FALSE
        }
    }

    pub unsafe fn add_(self, rhs: Self) -> (Self, bool) {
        (self.type_pointer.add)(self.value, rhs)
    }

    pub unsafe fn sub_(self, rhs: Self) -> (Self, bool) {
        (self.type_pointer.sub)(self.value, rhs)
    }

    pub unsafe fn mul_(self, rhs: Self) -> (Self, bool) {
        (self.type_pointer.mul)(self.value, rhs)
    }

    pub unsafe fn div_(self, rhs: Self) -> (Self, bool) {
        (self.type_pointer.div)(self.value, rhs)
    }

    pub unsafe fn rem_(self, rhs: Self) -> (Self, bool) {
        (self.type_pointer.rem)(self.value, rhs)
    }

    pub unsafe fn exp_(self, rhs: Self) -> (Self, bool) {
        (self.type_pointer.exp)(self.value, rhs)
    }

    pub unsafe fn eqeq_(self, rhs: Self) -> Self {
        (self.type_pointer.eqeq)(self.value, rhs).0
    }

    pub unsafe fn gt_(self, rhs: Self) -> (Self, bool) {
        (self.type_pointer.gt)(self.value, rhs)
    }

    pub unsafe fn gteq_(self, rhs: Self) -> (Self, bool) {
        (self.type_pointer.gteq)(self.value, rhs)
    }

    pub unsafe fn lt_(self, rhs: Self) -> (Self, bool) {
        (self.type_pointer.lt)(self.value, rhs)
    }

    pub unsafe fn lteq_(self, rhs: Self) -> (Self, bool) {
        (self.type_pointer.lteq)(self.value, rhs)
    }

    pub unsafe fn In_(self, rhs: Self) -> (Self, bool) {
        (self.type_pointer.In)(self.value, rhs)
    }

    pub unsafe fn instanceof_(self, rhs: Self) -> (Self, bool) {
        (self.type_pointer.instanceOf)(self.value, rhs)
    }

    pub unsafe fn remove_key_static_(self, id: u32) {
        (self.type_pointer.remove_key_static)(self.value, id);
    }

    pub fn type_str(self) -> &'static str {
        if self.is_bigint() {
            "bigint"
        } else if self.is_bool() {
            "boolean"
        } else if self.is_number() {
            "number"
        } else if self.is_string() {
            "string"
        } else if self.is_symbol() {
            "symbol"
        } else if self.is_undefined() {
            "undefined"
        } else {
            "object"
        }
    }

    /// called by the jitted code
    pub unsafe fn call_(
        self,
        runtime: &Runtime,
        this: JValue,
        argv: *const TempAllocValue,
        argc: u32,
        stack: *mut JValue,
    ) -> (JValue, bool) {
        let mut len = 0;
        let args = std::slice::from_raw_parts(argv, argc as usize);

        for i in args {
            // spread
            if i.flag == 1 {
                let iter = FastIterator::new(i.value, crate::bytecodes::LoopHint::ForOf);

                loop {
                    let (done, error, value) = iter.next(this, stack);
                    if error {
                        (value, error);
                    }
                    let ptr = (stack as *mut JValue).add(len as usize);
                    *ptr = value;
                    len += 1;
                    if done {
                        break;
                    }
                }
                iter.drop_();
            } else {
                let ptr = (stack as *mut JValue).add(len as usize);
                *ptr = i.value;
                len += 1;
            }
        }
        self.call_raw(runtime, this, stack, len)
    }

    #[inline]
    pub unsafe fn call_raw(
        self,
        runtime: &Runtime,
        this: JValue,
        stack: *mut JValue,
        argc: u32,
    ) -> (JValue, bool) {
        
        if self.is_object() {
            
            self.value.object.call(runtime, this, stack, argc)
        } else {
            println!("call");
            (JValue::Error(Error::CallOnNonFunction), true)
        }
    }

    pub fn call(self, ctx: JSFuncContext, this: JValue, args: &[JValue]) -> Result<JValue, JValue> {
        if !self.is_object() {
            return Err(JValue::Error(Error::TypeError(
                "cannot call on non function".to_owned(),
            )));
        }
        let runtime = Runtime::current();
        let (result, error) = unsafe {
            std::ptr::copy(args.as_ptr(), ctx.stack as *mut JValue, args.len());
            self.call_raw(&runtime, this, ctx.stack, args.len() as u32)
        };
        if error {
            Err(result)
        } else {
            Ok(result)
        }
    }

    pub unsafe fn new_raw(
        self,
        runtime: &Runtime,
        stack: *mut JValue,
        argc: u32,
    ) -> (JValue, bool) {
        todo!()
    }

    pub fn get_property(self, field: JValue) -> Result<JValue, JValue> {
        let key = field.to_string();
        self.get_property_str(&key)
    }

    pub fn get_property_str(self, field: &str) -> Result<JValue, JValue> {
        let rt = Runtime::current();
        let id = rt.register_field_name(field);
        let (result, error) = self.get_property_static_(id);
        if error {
            Err(result)
        } else {
            Ok(result)
        }
    }

    pub(crate) fn get_property_static_(self, field_id: u32) -> (JValue, bool) {
        todo!()
    }

    pub(crate) fn get_property_raw(self, field_id: u32, stack: *mut JValue) -> (JValue, bool) {
        unsafe { (self.type_pointer.get_static)(self.value, field_id, stack) }
    }

    pub fn set_property(self, field: JValue) -> Result<JValue, JValue> {
        let key = field.to_string();
        self.set_property_str(&key)
    }

    pub fn set_property_str(self, key: &str) -> Result<JValue, JValue> {
        let runtime = Runtime::current();
        let id = runtime.register_field_name(key);

        let (result, error) = self.set_property_static(id);
        if error {
            Err(result)
        } else {
            Ok(result)
        }
    }

    pub fn set_property_static(self, field_id: u32) -> (JValue, bool) {
        todo!()
    }

    pub(crate) fn set_property_raw(
        self,
        field_id: u32,
        value: JValue,
        stack: *mut JValue,
    ) -> (JValue, bool) {
        unsafe { (self.type_pointer.set_static)(self.value, field_id, value, stack) }
    }

    pub(crate) unsafe fn trace(self) {
        if self.is_object() {
            self.value.object.trace();
        } else if self.is_string() {
        }
    }
}

impl std::ops::Add for JValue {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        unsafe {
            if self.is_bigint() {
                bigint::add(self.value, rhs)
            } else if self.is_true() {
                true_::add(self.value, rhs)
            } else if self.is_false() {
                false_::add(self.value, rhs)
            } else if self.is_null() {
                null::add(self.value, rhs)
            } else if self.is_number() {
                number::add(self.value, rhs)
            } else if self.is_object() {
                object::add(self.value, rhs)
            } else if self.is_string() {
                string::add(self.value, rhs)
            } else if self.is_symbol() {
                todo!("throw when add symbol")
            } else {
                // undefined
                undefined::add(self.value, rhs)
            }
            .0
        }
    }
}

impl std::ops::Sub for JValue {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        unsafe {
            if self.is_bigint() {
                bigint::sub(self.value, rhs)
            } else if self.is_true() {
                true_::sub(self.value, rhs)
            } else if self.is_false() {
                false_::sub(self.value, rhs)
            } else if self.is_null() {
                null::sub(self.value, rhs)
            } else if self.is_number() {
                number::sub(self.value, rhs)
            } else if self.is_object() {
                object::sub(self.value, rhs)
            } else if self.is_string() {
                string::sub(self.value, rhs)
            } else if self.is_symbol() {
                todo!("throw when sub symbol")
            } else {
                // undefined
                undefined::sub(self.value, rhs)
            }
            .0
        }
    }
}

impl std::ops::Mul for JValue {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self::Output {
        unsafe {
            if self.is_bigint() {
                bigint::mul(self.value, rhs)
            } else if self.is_true() {
                true_::mul(self.value, rhs)
            } else if self.is_false() {
                false_::mul(self.value, rhs)
            } else if self.is_null() {
                null::mul(self.value, rhs)
            } else if self.is_number() {
                number::mul(self.value, rhs)
            } else if self.is_object() {
                object::mul(self.value, rhs)
            } else if self.is_string() {
                string::mul(self.value, rhs)
            } else if self.is_symbol() {
                todo!("throw when multiplying symbol")
            } else {
                // undefined
                undefined::mul(self.value, rhs)
            }
            .0
        }
    }
}

impl std::ops::Div for JValue {
    type Output = Self;
    fn div(self, rhs: Self) -> Self::Output {
        unsafe {
            if self.is_bigint() {
                bigint::div(self.value, rhs)
            } else if self.is_true() {
                true_::div(self.value, rhs)
            } else if self.is_false() {
                false_::div(self.value, rhs)
            } else if self.is_null() {
                null::div(self.value, rhs)
            } else if self.is_number() {
                number::div(self.value, rhs)
            } else if self.is_object() {
                object::div(self.value, rhs)
            } else if self.is_string() {
                string::div(self.value, rhs)
            } else if self.is_symbol() {
                todo!("throw when div symbol")
            } else {
                // undefined
                undefined::div(self.value, rhs)
            }
            .0
        }
    }
}

impl std::ops::Rem for JValue {
    type Output = Self;
    fn rem(self, rhs: Self) -> Self::Output {
        unsafe {
            if self.is_bigint() {
                bigint::rem(self.value, rhs)
            } else if self.is_true() {
                true_::rem(self.value, rhs)
            } else if self.is_false() {
                false_::rem(self.value, rhs)
            } else if self.is_null() {
                null::rem(self.value, rhs)
            } else if self.is_number() {
                number::rem(self.value, rhs)
            } else if self.is_object() {
                object::rem(self.value, rhs)
            } else if self.is_string() {
                string::rem(self.value, rhs)
            } else if self.is_symbol() {
                todo!("throw when rem symbol")
            } else {
                // undefined
                undefined::rem(self.value, rhs)
            }
            .0
        }
    }
}

impl PartialEq for JValue {
    fn eq(&self, other: &Self) -> bool {
        unsafe {
            std::mem::transmute_copy::<_, [usize; 2]>(self)
                == std::mem::transmute_copy::<_, [usize; 2]>(other)
        }
    }
}

impl Eq for JValue {}

impl std::cmp::PartialOrd for JValue {
    fn ge(&self, other: &Self) -> bool {
        self.to_number() >= other.to_number()
    }

    fn gt(&self, other: &Self) -> bool {
        self.to_number() > other.to_number()
    }

    fn le(&self, other: &Self) -> bool {
        self.to_number() <= other.to_number()
    }

    fn lt(&self, other: &Self) -> bool {
        self.to_number() < other.to_number()
    }

    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        if self > other {
            Some(std::cmp::Ordering::Greater)
        } else if self < other {
            Some(std::cmp::Ordering::Less)
        } else {
            Some(std::cmp::Ordering::Equal)
        }
    }
}

impl std::ops::Not for JValue {
    type Output = Self;
    fn not(self) -> Self::Output {
        (!self.to_bool()).into()
    }
}

impl std::ops::Neg for JValue {
    type Output = Self;
    fn neg(self) -> Self::Output {
        (-self.to_number()).into()
    }
}

impl std::ops::Shl for JValue {
    type Output = Self;
    fn shl(self, rhs: Self) -> Self::Output {
        JValue::Number(((self.to_number() as i32) << (rhs.to_number() as i32)) as f64)
    }
}

impl std::ops::Shr for JValue {
    type Output = Self;
    fn shr(self, rhs: Self) -> Self::Output {
        JValue::Number(((self.to_number() as i32) >> (rhs.to_number() as i32)) as f64)
    }
}

impl std::ops::BitAnd for JValue {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self::Output {
        JValue::Number((self.to_i32() & rhs.to_i32()) as f64)
    }
}

impl std::ops::BitOr for JValue {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self::Output {
        JValue::Number((self.to_i32() | rhs.to_i32()) as f64)
    }
}

impl std::ops::BitXor for JValue {
    type Output = Self;
    fn bitxor(self, rhs: Self) -> Self::Output {
        JValue::Number((self.to_i32() ^ rhs.to_i32()) as f64)
    }
}

impl ToString for JValue {
    fn to_string(&self) -> String {
        unsafe {
            if self.is_string() {
                self.value.string.to_string()
            } else if self.is_object() {
                self.value.object.to_string()
            } else if self.is_bigint() {
                self.value.bigint.to_string()
            } else if self.is_false() {
                "false".to_string()
            } else if self.is_null() {
                "null".to_string()
            } else if self.is_number() {
                self.value.number.to_string()
            } else if self.is_symbol() {
                self.value.symbol.to_string()
            } else if self.is_true() {
                "true".to_string()
            } else {
                "undefined".to_string()
            }
        }
    }
}

mod number {
    use crate::error::Error;

    use super::{JValue, JValueUnion};

    pub(crate) unsafe fn add(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        let v = if rhs.is_number() {
            JValue::Number(value.number + rhs.as_number_uncheck())
        } else if rhs.is_string() {
            JValue::String((value.number.to_string() + rhs.value.string).into())
        } else if rhs.is_false() {
            JValue::Number(value.number)
        } else if rhs.is_null() {
            JValue::Number(value.number)
        } else if rhs.is_undefined() {
            JValue::Number(f64::NAN)
        } else if rhs.is_true() {
            JValue::Number(value.number + 1.0)
        } else if rhs.is_object() {
            JValue::String(
                (value.number.to_string() + rhs.value.object.to_string().as_str()).into(),
            )
        } else if rhs.is_bigint() {
            JValue::Number(value.number + rhs.value.bigint as f64)
        } else {
            // symbol
            // todo: throw TypeError: cannot convert symbol to primitive
            return super::symbol::throw(value, rhs);
        };
        (v, false)
    }

    pub(crate) unsafe fn sub(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        let v = if rhs.is_number() {
            JValue::Number(value.number - rhs.value.number)
        } else if rhs.is_bigint() {
            JValue::Number(value.number - rhs.value.bigint as f64)
        } else if rhs.is_false() {
            JValue::Number(value.number)
        } else if rhs.is_null() {
            JValue::Number(value.number)
        } else if rhs.is_undefined() {
            JValue::Number(f64::NAN)
        } else if rhs.is_true() {
            JValue::Number(value.number - 1.0)
        } else if rhs.is_string() {
            if let Ok(v) = rhs.value.string.parse::<f64>() {
                JValue::Number(value.number - v)
            } else {
                JValue::Number(f64::NAN)
            }
        } else if rhs.is_symbol() {
            return super::symbol::throw(value, rhs);
        } else {
            JValue::Number(f64::NAN)
        };
        (v, false)
    }

    pub(crate) unsafe fn mul(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        let v = if rhs.is_number() {
            JValue::Number(value.number * rhs.value.number)
        } else if rhs.is_bigint() {
            JValue::Number(value.number * rhs.value.bigint as f64)
        } else if rhs.is_false() {
            JValue::Number(0.0)
        } else if rhs.is_null() {
            JValue::Number(0.0)
        } else if rhs.is_undefined() {
            JValue::Number(f64::NAN)
        } else if rhs.is_true() {
            JValue::Number(1.0)
        } else if rhs.is_string() {
            if let Ok(v) = rhs.value.string.parse::<f64>() {
                JValue::Number(value.number * v)
            } else {
                JValue::Number(f64::NAN)
            }
        } else if rhs.is_symbol() {
            return super::symbol::throw(value, rhs);
        } else {
            JValue::Number(f64::NAN)
        };
        (v, false)
    }

    pub(crate) unsafe fn div(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        let v = if rhs.is_number() {
            JValue::Number(value.number / rhs.value.number)
        } else if rhs.is_bigint() {
            JValue::Number(value.number / rhs.value.bigint as f64)
        } else if rhs.is_false() {
            JValue::Number(f64::INFINITY)
        } else if rhs.is_null() {
            JValue::Number(f64::INFINITY)
        } else if rhs.is_undefined() {
            JValue::Number(f64::NAN)
        } else if rhs.is_true() {
            JValue::Number(value.number)
        } else if rhs.is_string() {
            if let Ok(v) = rhs.value.string.parse::<f64>() {
                JValue::Number(value.number / v)
            } else {
                JValue::Number(f64::NAN)
            }
        } else if rhs.is_symbol() {
            return super::symbol::throw(value, rhs);
        } else {
            JValue::Number(f64::NAN)
        };
        (v, false)
    }

    pub(crate) unsafe fn rem(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        let v = if rhs.is_number() {
            JValue::Number(value.number % rhs.value.number)
        } else if rhs.is_bigint() {
            JValue::Number(value.number % rhs.value.bigint as f64)
        } else if rhs.is_false() {
            JValue::Number(f64::NAN)
        } else if rhs.is_null() {
            JValue::Number(f64::NAN)
        } else if rhs.is_undefined() {
            JValue::Number(f64::NAN)
        } else if rhs.is_true() {
            JValue::Number(value.number % 1.0)
        } else if rhs.is_string() {
            if let Ok(v) = rhs.value.string.parse::<f64>() {
                JValue::Number(value.number % v)
            } else {
                JValue::Number(f64::NAN)
            }
        } else if rhs.is_symbol() {
            return super::symbol::throw(value, rhs);
        } else {
            JValue::Number(f64::NAN)
        };
        (v, false)
    }

    pub(crate) unsafe fn exp(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        let v = if rhs.is_number() {
            JValue::Number(value.number.powf(rhs.value.number))
        } else if rhs.is_bigint() {
            JValue::Number(value.number.powf(rhs.value.bigint as f64))
        } else if rhs.is_false() {
            JValue::Number(f64::NAN)
        } else if rhs.is_null() {
            JValue::Number(f64::NAN)
        } else if rhs.is_undefined() {
            JValue::Number(f64::NAN)
        } else if rhs.is_true() {
            JValue::Number(value.number)
        } else if rhs.is_string() {
            if let Ok(v) = rhs.value.string.parse::<f64>() {
                JValue::Number(1.0)
            } else {
                JValue::Number(f64::NAN)
            }
        } else if rhs.is_symbol() {
            return super::symbol::throw(value, rhs);
        } else {
            JValue::Number(f64::NAN)
        };
        (v, false)
    }

    pub(crate) unsafe fn eqeq(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        let v = if rhs.is_number() {
            value.number == rhs.value.number
        } else if rhs.is_bigint() {
            value.number == rhs.value.bigint as f64
        } else if rhs.is_false() {
            value.number == 0.0
        } else if rhs.is_null() {
            value.number == 0.0
        } else if rhs.is_undefined() {
            value.number == f64::NAN
        } else if rhs.is_true() {
            value.number == 1.0
        } else if rhs.is_string() {
            if let Ok(v) = rhs.value.string.parse::<f64>() {
                value.number == v
            } else {
                false
            }
        } else if rhs.is_symbol() {
            false
        } else {
            false
        };
        if v {
            (JValue::TRUE, false)
        } else {
            (JValue::FALSE, false)
        }
    }

    pub(crate) unsafe fn noteq(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        let (v, error) = eqeq(value, rhs);
        if error {
            return (v, true);
        }
        if v.is_true() {
            (JValue::FALSE, false)
        } else {
            (JValue::TRUE, false)
        }
    }

    pub(crate) unsafe fn gt(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        let v = if rhs.is_number() {
            value.number > rhs.value.number
        } else if rhs.is_bigint() {
            value.number as i64 > rhs.value.bigint
        } else if rhs.is_false() {
            value.number > 0.0
        } else if rhs.is_true() {
            value.number > 1.0
        } else if rhs.is_null() {
            value.number > 0.0
        } else if rhs.is_object() {
            false
        } else if rhs.is_string() {
            if rhs.value.string.len() == 0 {
                value.number > 0.0
            } else {
                if let Ok(v) = rhs.value.string.parse::<f64>() {
                    value.number > v
                } else {
                    false
                }
            }
        } else if rhs.is_undefined() {
            value.number > f64::NAN
        } else {
            false
        };

        if v {
            (JValue::TRUE, false)
        } else {
            (JValue::FALSE, false)
        }
    }

    pub(crate) unsafe fn gteq(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        let v = if rhs.is_number() {
            value.number >= rhs.value.number
        } else if rhs.is_bigint() {
            value.number as i64 >= rhs.value.bigint
        } else if rhs.is_false() {
            value.number >= 0.0
        } else if rhs.is_true() {
            value.number >= 1.0
        } else if rhs.is_null() {
            value.number >= 0.0
        } else if rhs.is_object() {
            false
        } else if rhs.is_string() {
            if rhs.value.string.len() == 0 {
                value.number >= 0.0
            } else {
                if let Ok(v) = rhs.value.string.parse::<f64>() {
                    value.number >= v
                } else {
                    false
                }
            }
        } else if rhs.is_undefined() {
            value.number > f64::NAN
        } else {
            false
        };

        if v {
            (JValue::TRUE, false)
        } else {
            (JValue::FALSE, false)
        }
    }

    pub(crate) unsafe fn lt(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        let v = if rhs.is_number() {
            value.number < rhs.value.number
        } else if rhs.is_bigint() {
            (value.number as i64) < (rhs.value.bigint)
        } else if rhs.is_false() {
            value.number < 0.0
        } else if rhs.is_true() {
            value.number < 1.0
        } else if rhs.is_null() {
            value.number < 0.0
        } else if rhs.is_object() {
            false
        } else if rhs.is_string() {
            if rhs.value.string.len() == 0 {
                value.number < 0.0
            } else {
                if let Ok(v) = rhs.value.string.parse::<f64>() {
                    value.number < v
                } else {
                    false
                }
            }
        } else if rhs.is_undefined() {
            value.number < f64::NAN
        } else {
            false
        };

        if v {
            (JValue::TRUE, false)
        } else {
            (JValue::FALSE, false)
        }
    }

    pub(crate) unsafe fn lteq(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        let v = if rhs.is_number() {
            value.number <= rhs.value.number
        } else if rhs.is_bigint() {
            value.number as i64 <= rhs.value.bigint
        } else if rhs.is_false() {
            value.number <= 0.0
        } else if rhs.is_true() {
            value.number <= 1.0
        } else if rhs.is_null() {
            value.number <= 0.0
        } else if rhs.is_object() {
            false
        } else if rhs.is_string() {
            if rhs.value.string.len() == 0 {
                value.number <= 0.0
            } else {
                if let Ok(v) = rhs.value.string.parse::<f64>() {
                    value.number <= v
                } else {
                    false
                }
            }
        } else if rhs.is_undefined() {
            value.number <= f64::NAN
        } else {
            false
        };

        if v {
            (JValue::TRUE, false)
        } else {
            (JValue::FALSE, false)
        }
    }
}

mod null {
    use super::JValue;
    use super::JValueUnion;

    pub(crate) unsafe fn add(_value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        let v = if rhs.is_number() {
            rhs
        } else if rhs.is_bigint() {
            rhs
        } else if rhs.is_false() {
            JValue::Number(0.0)
        } else if rhs.is_undefined() {
            JValue::Number(0.0)
        } else if rhs.is_true() {
            JValue::Number(1.0)
        } else if rhs.is_string() {
            JValue::String(format!("null{}", rhs.value.string.as_ref()).into())
        } else if rhs.is_object() {
            JValue::String(format!("null{}", rhs.value.object.to_string()).into())
        } else {
            // symbol
            JValue::Number(f64::NAN)
        };
        (v, false)
    }

    pub(crate) unsafe fn sub(_value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        return super::number::sub(JValueUnion { number: 0.0 }, rhs);
    }

    pub(crate) unsafe fn mul(_value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        let v = if rhs.is_string() {
            if let Ok(_v) = rhs.value.string.parse::<f64>() {
                JValue::Number(0.0)
            } else {
                JValue::Number(f64::NAN)
            }
        } else if rhs.is_object() {
            JValue::Number(f64::NAN)
        } else if rhs.is_symbol() {
            // todo: throw TypeError
            JValue::Number(f64::NAN)
        } else {
            JValue::Number(0.0)
        };
        (v, false)
    }

    pub(crate) unsafe fn div(_value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        return super::number::div(JValueUnion { number: 0.0 }, rhs);
    }

    pub(crate) unsafe fn rem(_value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        return super::number::rem(JValueUnion { number: 0.0 }, rhs);
    }

    pub(crate) unsafe fn exp(_value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        return super::number::exp(JValueUnion { number: 0.0 }, rhs);
    }

    pub(crate) unsafe fn eqeq(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        let v = if rhs.is_null() {
            JValue::TRUE
        } else if rhs.is_undefined() {
            JValue::TRUE
        } else {
            JValue::FALSE
        };
        (v, false)
    }

    pub(crate) unsafe fn noteq(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        let v = if rhs.is_null() {
            JValue::FALSE
        } else if rhs.is_undefined() {
            JValue::FALSE
        } else {
            JValue::TRUE
        };
        (v, false)
    }

    pub(crate) unsafe fn gt(_value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        super::number::gt(JValueUnion { number: 0.0 }, rhs)
    }

    pub(crate) unsafe fn gteq(_value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        super::number::gteq(JValueUnion { number: 0.0 }, rhs)
    }

    pub(crate) unsafe fn lt(_value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        super::number::lteq(JValueUnion { number: 0.0 }, rhs)
    }

    pub(crate) unsafe fn lteq(_value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        super::number::lteq(JValueUnion { number: 0.0 }, rhs)
    }
}

mod undefined {
    use super::{JValue, JValueUnion};

    pub(crate) unsafe fn add(_value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        let v = if rhs.is_string() {
            JValue::String(format!("undefined{}", rhs.value.string.as_ref()).into())
        } else if rhs.is_object() {
            JValue::String(format!("undefined{}", rhs.value.object.to_string()).into())
        } else if rhs.is_symbol() {
            // symbol
            JValue::Number(f64::NAN)
        } else {
            JValue::Number(f64::NAN)
        };
        (v, false)
    }

    pub(crate) unsafe fn sub(_value: JValueUnion, _rhs: JValue) -> (JValue, bool) {
        return (JValue::Number(f64::NAN), false);
    }

    pub(crate) unsafe fn mul(_value: JValueUnion, _rhs: JValue) -> (JValue, bool) {
        return (JValue::Number(f64::NAN), false);
    }

    pub(crate) unsafe fn div(_value: JValueUnion, _rhs: JValue) -> (JValue, bool) {
        return (JValue::Number(f64::NAN), false);
    }

    pub(crate) unsafe fn rem(_value: JValueUnion, _rhs: JValue) -> (JValue, bool) {
        return (JValue::Number(f64::NAN), false);
    }

    pub(crate) unsafe fn exp(_value: JValueUnion, _rhs: JValue) -> (JValue, bool) {
        return (JValue::Number(f64::NAN), false);
    }

    pub(crate) unsafe fn eqeq(_value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        if rhs.is_undefined() {
            (JValue::TRUE, false)
        } else if rhs.is_null() {
            (JValue::TRUE, false)
        } else {
            (JValue::FALSE, false)
        }
    }

    pub(crate) unsafe fn noteq(_value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        if rhs.is_undefined() {
            (JValue::FALSE, false)
        } else if rhs.is_null() {
            (JValue::FALSE, false)
        } else {
            (JValue::TRUE, false)
        }
    }

    pub(crate) unsafe fn gt(_value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        super::number::gt(JValueUnion { number: f64::NAN }, rhs)
    }

    pub(crate) unsafe fn gteq(_value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        super::number::gteq(JValueUnion { number: f64::NAN }, rhs)
    }

    pub(crate) unsafe fn lt(_value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        super::number::lt(JValueUnion { number: f64::NAN }, rhs)
    }

    pub(crate) unsafe fn lteq(_value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        super::number::lteq(JValueUnion { number: f64::NAN }, rhs)
    }
}

mod true_ {
    use super::{JValue, JValueUnion};

    pub(crate) unsafe fn add(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        let v = if rhs.is_string() {
            JValue::String(format!("true{}", rhs.value.string.as_ref()).into())
        } else if rhs.is_object() {
            JValue::String(format!("true{}", rhs.value.object.to_string()).into())
        } else if rhs.is_symbol() {
            return super::symbol::throw(value, rhs);
        } else {
            return super::number::add(JValueUnion { number: 1.0 }, rhs);
        };
        (v, false)
    }

    pub(crate) unsafe fn sub(_value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        return super::number::sub(JValueUnion { number: 1.0 }, rhs);
    }

    pub(crate) unsafe fn mul(_value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        return super::number::mul(JValueUnion { number: 1.0 }, rhs);
    }

    pub(crate) unsafe fn div(_value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        return super::number::div(JValueUnion { number: 1.0 }, rhs);
    }

    pub(crate) unsafe fn rem(_value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        return super::number::sub(JValueUnion { number: 1.0 }, rhs);
    }

    pub(crate) unsafe fn exp(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        super::number::exp(JValueUnion { number: 1.0 }, rhs)
    }

    pub(crate) unsafe fn eqeq(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        super::number::eqeq(JValueUnion { number: 1.0 }, rhs)
    }

    pub(crate) unsafe fn noteq(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        super::number::noteq(JValueUnion { number: 1.0 }, rhs)
    }

    pub(crate) unsafe fn gt(_value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        super::number::gt(JValueUnion { number: 1.0 }, rhs)
    }

    pub(crate) unsafe fn gteq(_value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        super::number::gteq(JValueUnion { number: 1.0 }, rhs)
    }

    pub(crate) unsafe fn lt(_value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        super::number::lteq(JValueUnion { number: 1.0 }, rhs)
    }

    pub(crate) unsafe fn lteq(_value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        super::number::lteq(JValueUnion { number: 1.0 }, rhs)
    }
}

mod false_ {
    use super::{JValue, JValueUnion};

    pub(crate) unsafe fn add(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        let v = if rhs.is_string() {
            JValue::String(format!("false{}", rhs.value.string.as_ref()).into())
        } else if rhs.is_object() {
            JValue::String(format!("false{}", rhs.value.object.to_string()).into())
        } else if rhs.is_symbol() {
            return super::symbol::throw(value, rhs);
        } else {
            return super::number::add(JValueUnion { number: 0.0 }, rhs);
        };
        (v, false)
    }

    pub(crate) unsafe fn sub(_value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        return super::number::sub(JValueUnion { number: 0.0 }, rhs);
    }

    pub(crate) unsafe fn mul(_value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        return super::number::mul(JValueUnion { number: 0.0 }, rhs);
    }

    pub(crate) unsafe fn div(_value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        return super::number::div(JValueUnion { number: 0.0 }, rhs);
    }

    pub(crate) unsafe fn rem(_value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        return super::number::sub(JValueUnion { number: 0.0 }, rhs);
    }

    pub(crate) unsafe fn exp(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        super::number::exp(JValueUnion { number: 0.0 }, rhs)
    }

    pub(crate) unsafe fn eqeq(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        super::number::eqeq(JValueUnion { number: 0.0 }, rhs)
    }

    pub(crate) unsafe fn noteq(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        super::number::noteq(JValueUnion { number: 0.0 }, rhs)
    }

    pub(crate) unsafe fn gt(_value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        super::number::gt(JValueUnion { number: 0.0 }, rhs)
    }

    pub(crate) unsafe fn gteq(_value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        super::number::gteq(JValueUnion { number: 0.0 }, rhs)
    }

    pub(crate) unsafe fn lt(_value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        super::number::lteq(JValueUnion { number: 0.0 }, rhs)
    }

    pub(crate) unsafe fn lteq(_value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        super::number::lteq(JValueUnion { number: 0.0 }, rhs)
    }
}

mod string {
    use crate::runtime::Runtime;

    use super::{JValue, JValueUnion};

    pub(crate) unsafe fn add(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        let v = if rhs.is_string() {
            JValue::String(value.string + rhs.value.string)
        } else if rhs.is_number() {
            JValue::String(value.string + rhs.value.number.to_string().as_ref())
        } else if rhs.is_bigint() {
            JValue::String(value.string + rhs.value.bigint.to_string().as_ref())
        } else if rhs.is_false() {
            JValue::String(value.string + "false")
        } else if rhs.is_true() {
            JValue::String(value.string + "true")
        } else if rhs.is_null() {
            JValue::String(value.string + "null")
        } else if rhs.is_undefined() {
            JValue::String(value.string + "undefined")
        } else if rhs.is_object() {
            JValue::String(value.string + rhs.value.object.to_string().as_ref())
        } else {
            // symbol
            // todo: throw TypeError
            JValue::String("".into())
        };
        (v, false)
    }

    pub(crate) unsafe fn sub(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        if value.string.len() == 0 {
            return super::number::sub(JValueUnion { number: 0.0 }, rhs);
        }
        if let Ok(v) = value.string.parse::<f64>() {
            return super::number::sub(JValueUnion { number: v }, rhs);
        } else {
            (JValue::Number(f64::NAN), false)
        }
    }

    pub(crate) unsafe fn mul(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        if value.string.len() == 0 {
            return super::number::mul(JValueUnion { number: 0.0 }, rhs);
        }
        if let Ok(v) = value.string.parse::<f64>() {
            return super::number::mul(JValueUnion { number: v }, rhs);
        } else {
            (JValue::Number(f64::NAN), false)
        }
    }

    pub(crate) unsafe fn div(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        if value.string.len() == 0 {
            return super::number::div(JValueUnion { number: 0.0 }, rhs);
        }
        if let Ok(v) = value.string.parse::<f64>() {
            return super::number::div(JValueUnion { number: v }, rhs);
        } else {
            (JValue::Number(f64::NAN), false)
        }
    }

    pub(crate) unsafe fn rem(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        if value.string.len() == 0 {
            return super::number::rem(JValueUnion { number: 0.0 }, rhs);
        }
        if let Ok(v) = value.string.parse::<f64>() {
            return super::number::rem(JValueUnion { number: v }, rhs);
        } else {
            (JValue::Number(f64::NAN), false)
        }
    }

    pub(crate) unsafe fn exp(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        if value.string.len() == 0 {
            return super::number::exp(JValueUnion { number: 0.0 }, rhs);
        }
        if let Ok(v) = value.string.parse::<f64>() {
            return super::number::exp(JValueUnion { number: v }, rhs);
        } else {
            (JValue::Number(f64::NAN), false)
        }
    }

    pub(crate) unsafe fn eqeq(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        let v = if rhs.is_bigint() {
            if let Ok(i) = value.string.parse::<i64>() {
                i == rhs.value.bigint
            } else {
                false
            }
        } else if rhs.is_false() || rhs.is_null() {
            if value.string.len() == 0 {
                true
            } else {
                if let Ok(v) = value.string.parse::<f64>() {
                    v == 0.0
                } else {
                    false
                }
            }
        } else if rhs.is_true() {
            if let Ok(v) = value.string.parse::<f64>() {
                v == 1.0
            } else {
                false
            }
        } else if rhs.is_object() {
            rhs.to_string().as_str() == value.string.as_ref()
        } else if rhs.is_string() {
            value.string.as_ref() == rhs.value.string.as_ref()
        } else if rhs.is_symbol() {
            false
        } else {
            false
        };

        if v {
            (JValue::TRUE, false)
        } else {
            (JValue::FALSE, false)
        }
    }

    pub(crate) unsafe fn noteq(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        let (v, error) = eqeq(value, rhs);
        if error {
            return (v, error);
        }
        if v.is_true() {
            (JValue::FALSE, false)
        } else {
            (JValue::TRUE, false)
        }
    }

    pub(crate) unsafe fn gt(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        if value.string.len() == 0 {
            super::number::gt(JValueUnion { number: 0.0 }, rhs)
        } else {
            if let Ok(v) = value.string.parse::<f64>() {
                super::number::gt(JValueUnion { number: v }, rhs)
            } else {
                (JValue::FALSE, false)
            }
        }
    }

    pub(crate) unsafe fn gteq(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        if value.string.len() == 0 {
            super::number::gteq(JValueUnion { number: 0.0 }, rhs)
        } else {
            if let Ok(v) = value.string.parse::<f64>() {
                super::number::gteq(JValueUnion { number: v }, rhs)
            } else {
                (JValue::FALSE, false)
            }
        }
    }

    pub(crate) unsafe fn lt(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        if value.string.len() == 0 {
            super::number::lt(JValueUnion { number: 0.0 }, rhs)
        } else {
            if let Ok(v) = value.string.parse::<f64>() {
                super::number::lt(JValueUnion { number: v }, rhs)
            } else {
                (JValue::FALSE, false)
            }
        }
    }

    pub(crate) unsafe fn lteq(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        if value.string.len() == 0 {
            super::number::lteq(JValueUnion { number: 0.0 }, rhs)
        } else {
            if let Ok(v) = value.string.parse::<f64>() {
                super::number::lteq(JValueUnion { number: v }, rhs)
            } else {
                (JValue::FALSE, false)
            }
        }
    }

    pub(crate) unsafe fn set(
        obj: JValueUnion,
        field: JValue,
        value: JValue,
        stack: *mut JValue,
    ) -> (JValue, bool) {
        (value, false)
    }

    pub(crate) unsafe fn set_static(
        obj: JValueUnion,
        field: u32,
        value: JValue,
        stack: *mut JValue,
    ) -> (JValue, bool) {
        (value, false)
    }

    pub(crate) unsafe fn remove_key_static(obj: JValueUnion, field: u32) {}

    pub(crate) unsafe fn get(
        obj: JValueUnion,
        field: JValue,
        stack: *mut JValue,
    ) -> (JValue, bool) {
        let v = if field.is_number() {
            let s = obj.string.as_ref().chars().nth(field.value.number as usize);
            match s {
                Some(v) => v.to_string().into(),
                None => JValue::UNDEFINED,
            }
        } else if field.is_bigint() {
            let s = obj.string.as_ref().chars().nth(field.value.bigint as usize);
            match s {
                Some(v) => v.to_string().into(),
                None => JValue::UNDEFINED,
            }
        } else if field.is_string() {
            if let Ok(v) = field.value.string.parse::<f64>() {
                let s = obj.string.as_ref().chars().nth(v as usize);
                match s {
                    Some(v) => v.to_string().into(),
                    None => JValue::UNDEFINED,
                }
            } else {
                JValue::UNDEFINED
            }
        } else {
            JValue::UNDEFINED
        };

        (v, false)
    }

    pub(crate) unsafe fn get_static(
        obj: JValueUnion,
        field: u32,
        stack: *mut JValue,
    ) -> (JValue, bool) {
        let runtime = Runtime::current();
        let field = runtime.get_field_name(field);

        if let Ok(n) = field.parse::<f64>() {
            if n < 0.0 {
                return (JValue::UNDEFINED, false);
            }
            let s = obj.string.as_ref().chars().nth(n as usize);
            match s {
                Some(v) => (v.to_string().into(), false),
                None => (JValue::UNDEFINED, false),
            }
        } else {
            (JValue::UNDEFINED, false)
        }
    }
}

mod bigint {
    use super::*;

    pub(crate) unsafe fn add(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        let v = if rhs.is_bigint() {
            JValue::BigInt(value.bigint + rhs.value.bigint)
        } else if rhs.is_string() {
            JValue::String((value.bigint.to_string() + rhs.value.string).into())
        } else {
            return (
                JValue::Error(Error::TypeError(
                    "TypeError: Cannot mix BigInt and other types, use explicit conversions".into(),
                )),
                true,
            );
        };
        (v, false)
    }

    pub(crate) unsafe fn sub(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        if rhs.is_bigint() {
            (JValue::BigInt(value.bigint - rhs.value.bigint), false)
        } else {
            (
                JValue::Error(Error::TypeError(
                    "TypeError: Cannot mix BigInt and other types, use explicit conversions"
                        .to_owned(),
                )),
                true,
            )
        }
    }

    pub(crate) unsafe fn mul(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        if rhs.is_bigint() {
            (JValue::BigInt(value.bigint * rhs.value.bigint), false)
        } else {
            (
                JValue::Error(Error::TypeError(
                    "TypeError: Cannot mix BigInt and other types, use explicit conversions"
                        .to_owned(),
                )),
                true,
            )
        }
    }

    pub(crate) unsafe fn div(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        if rhs.is_bigint() {
            (JValue::BigInt(value.bigint / rhs.value.bigint), false)
        } else {
            (
                JValue::Error(Error::TypeError(
                    "TypeError: Cannot mix BigInt and other types, use explicit conversions"
                        .to_owned(),
                )),
                true,
            )
        }
    }

    pub(crate) unsafe fn rem(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        if rhs.is_bigint() {
            (JValue::BigInt(value.bigint % rhs.value.bigint), false)
        } else {
            (
                JValue::Error(Error::TypeError(
                    "TypeError: Cannot mix BigInt and other types, use explicit conversions"
                        .to_owned(),
                )),
                true,
            )
        }
    }

    pub(crate) unsafe fn exp(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        if rhs.is_bigint() {
            (
                JValue::BigInt(value.bigint.pow(rhs.value.bigint as u32)),
                false,
            )
        } else {
            (
                JValue::Error(Error::TypeError(
                    "TypeError: Cannot mix BigInt and other types, use explicit conversions"
                        .to_owned(),
                )),
                true,
            )
        }
    }

    pub(crate) unsafe fn eqeq(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        let v = if rhs.is_bigint() {
            value.bigint == rhs.value.bigint
        } else if rhs.is_false() {
            value.bigint == 0
        } else if rhs.is_true() {
            value.bigint == 1
        } else if rhs.is_null() {
            value.bigint == 0
        } else if rhs.is_number() {
            value.bigint == rhs.value.number as i64
        } else if rhs.is_string() {
            if rhs.value.string.len() == 0 {
                value.bigint == 0
            } else {
                if let Ok(v) = rhs.value.string.parse::<i64>() {
                    value.bigint == v
                } else {
                    false
                }
            }
        } else {
            // object, symbol, undefined
            false
        };
        if v {
            (JValue::TRUE, false)
        } else {
            (JValue::FALSE, false)
        }
    }

    pub(crate) unsafe fn noteq(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        let (v, error) = eqeq(value, rhs);
        if error {
            return (v, true);
        }
        if v.is_true() {
            (JValue::FALSE, false)
        } else {
            (JValue::TRUE, false)
        }
    }

    pub(crate) unsafe fn gt(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        let v = if rhs.is_bigint() {
            value.bigint > rhs.value.bigint
        } else if rhs.is_false() {
            value.bigint > 0
        } else if rhs.is_true() {
            value.bigint > 1
        } else if rhs.is_null() {
            value.bigint > 0
        } else if rhs.is_number() {
            value.bigint > rhs.value.number as i64
        } else if rhs.is_string() {
            if rhs.value.string.len() == 0 {
                value.bigint > 0
            } else {
                if let Ok(v) = rhs.value.string.parse::<i64>() {
                    value.bigint > v
                } else {
                    false
                }
            }
        } else {
            // object, symbol, undefined
            false
        };
        if v {
            (JValue::TRUE, false)
        } else {
            (JValue::FALSE, false)
        }
    }

    pub(crate) unsafe fn gteq(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        let v = if rhs.is_bigint() {
            value.bigint >= rhs.value.bigint
        } else if rhs.is_false() {
            value.bigint >= 0
        } else if rhs.is_true() {
            value.bigint >= 1
        } else if rhs.is_null() {
            value.bigint >= 0
        } else if rhs.is_number() {
            value.bigint >= rhs.value.number as i64
        } else if rhs.is_string() {
            if rhs.value.string.len() == 0 {
                value.bigint >= 0
            } else {
                if let Ok(v) = rhs.value.string.parse::<i64>() {
                    value.bigint >= v
                } else {
                    false
                }
            }
        } else {
            // object, symbol, undefined
            false
        };
        if v {
            (JValue::TRUE, false)
        } else {
            (JValue::FALSE, false)
        }
    }

    pub(crate) unsafe fn lt(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        let v = if rhs.is_bigint() {
            value.bigint < rhs.value.bigint
        } else if rhs.is_false() {
            value.bigint < 0
        } else if rhs.is_true() {
            value.bigint < 1
        } else if rhs.is_null() {
            value.bigint < 0
        } else if rhs.is_number() {
            value.bigint < rhs.value.number as i64
        } else if rhs.is_string() {
            if rhs.value.string.len() == 0 {
                value.bigint < 0
            } else {
                if let Ok(v) = rhs.value.string.parse::<i64>() {
                    value.bigint < v
                } else {
                    false
                }
            }
        } else {
            // object, symbol, undefined
            false
        };
        if v {
            (JValue::TRUE, false)
        } else {
            (JValue::FALSE, false)
        }
    }

    pub(crate) unsafe fn lteq(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        let v = if rhs.is_bigint() {
            value.bigint <= rhs.value.bigint
        } else if rhs.is_false() {
            value.bigint <= 0
        } else if rhs.is_true() {
            value.bigint <= 1
        } else if rhs.is_null() {
            value.bigint <= 0
        } else if rhs.is_number() {
            value.bigint <= rhs.value.number as i64
        } else if rhs.is_string() {
            if rhs.value.string.len() == 0 {
                value.bigint <= 0
            } else {
                if let Ok(v) = rhs.value.string.parse::<i64>() {
                    value.bigint <= v
                } else {
                    false
                }
            }
        } else {
            // object, symbol, undefined
            false
        };
        if v {
            (JValue::TRUE, false)
        } else {
            (JValue::FALSE, false)
        }
    }
}

mod object {
    use super::{JValue, JValueUnion};

    pub(crate) unsafe fn add(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        if value.object.is_primitive() {
            value.object.to_primitive().unwrap().add_(rhs)
        } else {
            (
                (value.object.to_string() + rhs.to_string().as_str()).into(),
                false,
            )
        }
    }

    pub(crate) unsafe fn sub(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        if value.object.is_primitive() {
            value.object.to_primitive().unwrap().sub_(rhs)
        } else {
            (JValue::Number(f64::NAN), false)
        }
    }

    pub(crate) unsafe fn mul(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        if value.object.is_primitive() {
            value.object.to_primitive().unwrap().mul_(rhs)
        } else {
            (JValue::Number(f64::NAN), false)
        }
    }

    pub(crate) unsafe fn div(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        if value.object.is_primitive() {
            value.object.to_primitive().unwrap().div_(rhs)
        } else {
            (JValue::Number(f64::NAN), false)
        }
    }

    pub(crate) unsafe fn rem(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        if value.object.is_primitive() {
            value.object.to_primitive().unwrap().rem_(rhs)
        } else {
            (JValue::Number(f64::NAN), false)
        }
    }

    pub(crate) unsafe fn exp(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        if value.object.is_primitive() {
            value.object.to_primitive().unwrap().exp_(rhs)
        } else {
            (JValue::Number(f64::NAN), false)
        }
    }

    pub(crate) unsafe fn eqeq(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        let v: bool = if rhs.is_object() {
            value.object == rhs.value.object
        } else if value.object.is_primitive() {
            return (value.object.to_primitive().unwrap().eqeq_(rhs), false);
        } else {
            false
        };
        (JValue::from(v), false)
    }

    pub(crate) unsafe fn noteq(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        let (v, error) = eqeq(value, rhs);
        if error {
            return (v, true);
        }
        if v.is_true() {
            (JValue::FALSE, false)
        } else {
            (JValue::TRUE, false)
        }
    }

    pub(crate) unsafe fn gt(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        if value.object.is_primitive() {
            value.object.to_primitive().unwrap().gt_(rhs)
        } else {
            (JValue::FALSE, false)
        }
    }

    pub(crate) unsafe fn lt(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        if value.object.is_primitive() {
            value.object.to_primitive().unwrap().lt_(rhs)
        } else {
            (JValue::FALSE, false)
        }
    }

    pub(crate) unsafe fn gteq(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        if value.object.is_primitive() {
            value.object.to_primitive().unwrap().gteq_(rhs)
        } else if rhs.is_object() {
            (JValue::from(value.object == rhs.value.object), false)
        } else {
            (JValue::FALSE, false)
        }
    }

    pub(crate) unsafe fn lteq(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        if value.object.is_primitive() {
            value.object.to_primitive().unwrap().lteq_(rhs)
        } else if rhs.is_object() {
            (JValue::from(value.object == rhs.value.object), false)
        } else {
            (JValue::FALSE, false)
        }
    }

    pub(crate) unsafe fn get(
        obj: JValueUnion,
        field: JValue,
        stack: *mut JValue,
    ) -> (JValue, bool) {
        obj.object.inner.get_property(&field.to_string(), stack)
    }

    pub(crate) unsafe fn get_static(
        obj: JValueUnion,
        field: u32,
        stack: *mut JValue,
    ) -> (JValue, bool) {
        obj.object.inner.get_static(field, stack)
    }

    pub(crate) unsafe fn set(
        obj: JValueUnion,
        field: JValue,
        value: JValue,
        stack: *mut JValue,
    ) -> (JValue, bool) {
        obj.object
            .inner
            .to_mut()
            .set_property(&field.to_string(), value, stack)
    }

    pub(crate) unsafe fn set_static(
        obj: JValueUnion,
        field: u32,
        value: JValue,
        stack: *mut JValue,
    ) -> (JValue, bool) {
        obj.object.inner.set_static(field, value, stack)
    }

    pub(crate) unsafe fn remove_key_static(obj: JValueUnion, field: u32) {
        obj.object.inner.to_mut().remove_key_static(field);
    }

    pub(crate) unsafe fn instanceOf(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        let v: bool = if rhs.is_object() {
            if rhs.value.object.inner.wrapped_value.is_function() {
                let v = rhs
                    .value
                    .object
                    .get_property("prototype", std::ptr::null_mut())
                    .unwrap();
                value
                    .object
                    .get_property("__proto__", std::ptr::null_mut())
                    .unwrap()
                    == v
            } else {
                false
            }
        } else {
            return (JValue::FALSE, false);
        };
        (JValue::from(v), false)
    }

    pub(crate) unsafe fn In(obj: JValueUnion, field: JValue) -> (JValue, bool) {
        let v = obj.object.has_owned_property(&field.to_string());
        (JValue::from(v), false)
    }

    pub(crate) unsafe fn False(_value: JValueUnion, _rhs: JValue) -> (JValue, bool) {
        (JValue::FALSE, false)
    }
}

mod notObject {
    use super::*;

    pub(crate) fn get(value: JValueUnion, rhs: JValue, stack: *mut JValue) -> (JValue, bool) {
        (
            JValue::Error(Error::TypeError(
                "cannot read property of non object".into(),
            )),
            true,
        )
    }

    pub(crate) fn get_static(value: JValueUnion, rhs: u32, stack: *mut JValue) -> (JValue, bool) {
        (
            JValue::Error(Error::TypeError(
                "cannot read property of non object".into(),
            )),
            true,
        )
    }

    pub(crate) fn set(
        obj: JValueUnion,
        field: JValue,
        value: JValue,
        stack: *mut JValue,
    ) -> (JValue, bool) {
        (
            JValue::Error(Error::TypeError("cannot set property of non object".into())),
            true,
        )
    }

    pub(crate) fn set_static(
        obj: JValueUnion,
        field: u32,
        value: JValue,
        stack: *mut JValue,
    ) -> (JValue, bool) {
        (
            JValue::Error(Error::TypeError("cannot set property of non object".into())),
            true,
        )
    }

    pub(crate) fn remove_key_static(obj: JValueUnion, field: u32) {}

    pub(crate) unsafe fn instanceOf(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        if !rhs.is_object() {
            return (
                JValue::Error(Error::TypeError(
                    "Right-hand side of 'instanceof' is not callable".into(),
                )),
                true,
            );
        }
        (JValue::FALSE, false)
    }

    pub(crate) unsafe fn In(obj: JValueUnion, rhs: JValue) -> (JValue, bool) {
        (JValue::FALSE, false)
    }
}

mod symbol {
    use super::{JValue, JValueUnion};

    pub(crate) unsafe fn throw(_value: JValueUnion, _rhs: JValue) -> (JValue, bool) {
        todo!("TypeError: cannot convert Symbol to primitives.")
    }
}

#[test]
fn test_jvalue_size() {
    assert!(std::mem::size_of::<JValue>() == 16);
}
