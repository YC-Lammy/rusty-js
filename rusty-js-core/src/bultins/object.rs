use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use crate::error::Error;
use crate::runtime::GcFlag;
use crate::runtime::Runtime;
use crate::types::JValue;

use crate::utils::nohasher::NoHasherBuilder;

use super::class::JSClassInstance;
use super::generator::Generator;
use super::object_builder::ObjectBuilder;
use super::typed_array::TypedArray;
use super::function::JSFunctionInstance;
use super::promise::Promise;
use super::proxy::Proxy;
use super::regex::RegExp;
use super::strings::JString;
use super::symbol::JSymbol;

use super::prop::PropFlag;

#[derive(Hash, Clone, Copy, PartialEq, Eq)]
pub struct PropKey(u32);

#[derive(Clone, Copy)]
pub union PropCell {
    value: JValue,
    getter: JObject,
    setter: JObject,

    getsetter: (JObject, JObject),
}

pub type PropMap = HashMap<PropKey, (PropFlag, PropCell), NoHasherBuilder>;

#[allow(unused)]
/// helper function for hashing
fn hash_<T>(v: T) -> u64
where
    T: Hash,
{
    let mut h = DefaultHasher::new();
    v.hash(&mut h);
    h.finish()
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct JObject {
    pub(crate) inner: &'static JObjectInner,
}

impl PartialEq for JObject {
    fn eq(&self, other: &Self) -> bool {
        self.inner as *const _ == other.inner as *const _
    }
}

impl ToString for JObject {
    fn to_string(&self) -> String {
        if self.has_owned_property("toString") {}
        if let Some(v) = self.as_error() {
            return v.to_string();
        }
        return "[object Object]".to_string();
    }
}

impl JObject {
    pub fn new() -> Self {
        Self {
            inner: JObjectInner::new(),
        }
    }

    pub fn new_target() -> Self{
        let obj = Self::new();
        obj.inner.to_mut().wrapped_value = JObjectValue::NewTarget;
        return obj
    }

    pub fn is_new_target(&self) -> bool{
        match self.inner.wrapped_value {
            JObjectValue::NewTarget => true,
            _ => false
        }
    }

    pub fn array() -> Self {
        Self::with_value(JObjectValue::Array(Vec::new()))
    }

    pub fn with_array(a:Vec<(PropFlag, JValue)>) -> Self{
        Self::with_value(JObjectValue::Array(a))
    }

    pub fn new_map() -> Self{
        Self::with_value(JObjectValue::Map(Default::default()))
    }

    pub fn new_set() -> Self{
        Self::with_value(JObjectValue::Set(Default::default()))
    }

    pub fn weak_set() -> Self{
        Self::with_value(JObjectValue::WeakSet(Default::default()))
    }

    pub fn weak_map() -> Self{
        Self::with_value(JObjectValue::WeakMap(Default::default()))
    }

    /// todo: use f.create_object instead
    pub fn with_function(f: JSFunctionInstance) -> Self {
        f.create_object()
    }

    pub fn with_error(e: Error) -> Self {
        let obj = JObjectInner::new();
        obj.wrapped_value = JObjectValue::Error(e);
        Self { inner: obj }
    }

    pub fn with_promise(p: Promise) -> Self {
        Self::with_value(JObjectValue::Promise(p))
    }

    pub fn with_regex(r:Arc<RegExp>) -> Self{
        Self::with_value(JObjectValue::Regex(r))
    }

    pub fn with_number(n:f64) -> Self{
        Self::with_value(JObjectValue::Number(n))
    }

    /// with value is private
    fn with_value(value: JObjectValue) -> Self {
        let obj = JObjectInner::new();
        obj.wrapped_value = value;
        Self { inner: obj }
    }

    pub fn as_array(&self) -> Option<&mut Vec<(PropFlag, JValue)>> {
        match &mut self.inner.to_mut().wrapped_value {
            JObjectValue::Array(a) => Some(a),
            _ => None,
        }
    }

    pub fn is_array(&self) -> bool {
        self.inner.wrapped_value.is_array()
    }

    pub fn as_function_instance(&self) -> Option<&mut JSFunctionInstance> {
        match &mut self.inner.to_mut().wrapped_value {
            JObjectValue::Function(f) => Some(f),
            _ => None,
        }
    }

    pub fn is_function_instance(&self) -> bool {
        self.as_function_instance().is_some()
    }

    pub fn as_regexp(&self) -> Option<Arc<RegExp>> {
        match &mut self.inner.to_mut().wrapped_value {
            JObjectValue::Regex(r) => Some(r.clone()),
            _ => None,
        }
    }

    pub fn is_regexp(&self) -> bool {
        self.as_regexp().is_some()
    }

    pub fn as_promise(&self) -> Option<&mut Promise> {
        match &mut self.inner.to_mut().wrapped_value {
            JObjectValue::Promise(p) => Some(p),
            _ => None,
        }
    }

    pub fn is_promise(&self) -> bool {
        self.as_promise().is_some()
    }

    pub fn as_error(&self) -> Option<&Error> {
        match &self.inner.wrapped_value {
            JObjectValue::Error(e) => Some(e),
            _ => None,
        }
    }

    pub fn is_error(&self) -> bool{
        self.as_error().is_some()
    }

    pub fn as_number(&self) -> Option<f64>{
        match &self.inner.wrapped_value {
            JObjectValue::Number(n) => Some(*n),
            _ => None
        }
    }

    pub fn is_number(&self) -> bool{
        self.as_number().is_some()
    }

    pub fn as_class(&self) -> Option<&mut JSClassInstance>{
        match &mut self.inner.to_mut().wrapped_value{
            JObjectValue::Class(c) => Some(c),
            _ => None
        }
    }
    pub fn is_class(&self) -> bool{
        self.as_class().is_some()
    }

    pub fn is_primitive(&self) -> bool {
        self.inner.wrapped_value.is_primitive()
    }

    pub fn to_primitive(&self) -> Option<JValue> {
        None
    }

    pub fn has_owned_property(&self, key: &str) -> bool {
        self.inner.has_owned_property(key)
    }

    pub fn has_owned_property_static(&self, key: u32) -> bool {
        self.inner.has_owned_property_static(key)
    }

    pub fn get_property(&self, key: &str, stack: *mut JValue) -> Option<JValue> {
        self.inner.get(key, stack)
    }

    pub fn get_property_static(&self, key_id: u32, stack: *mut JValue) -> Option<JValue> {
        let (v, error) = unsafe { self.inner.get_property_static(key_id, stack) };
        if error {
            None
        } else {
            if v.is_undefined() {
                None
            } else {
                Some(v)
            }
        }
    }

    #[inline]
    pub fn insert_property(&mut self, key: &str, value: JValue, flag: PropFlag) {
        self.inner.to_mut().insert_property(key, value, flag)
    }

    pub fn insert_property_static(&mut self, key: u32, value: JValue, flag: PropFlag) {
        self.inner.to_mut().insert_property_static(key, value, flag)
    }

    pub fn set_property_static(&mut self, key_id: u32, value: JValue, stack: *mut JValue) {
        unsafe {
            self.inner
                .to_mut()
                .set_property_static(key_id, value, stack)
        };
    }

    pub fn bind_getter(&self, id: u32, getter: JObject) {
        if let Some((flag, cell)) = self.inner.to_mut().values.get_mut(&PropKey(id)) {
            if flag.is_getter() && flag.is_setter() {
                *cell = PropCell {
                    getsetter: (getter, unsafe { cell.getsetter.1 }),
                };
            } else if flag.is_setter() {
                *cell = PropCell {
                    getsetter: (getter, unsafe { cell.setter }),
                };
            } else {
                *cell = PropCell { getter: getter };
            };
            *flag = *flag | PropFlag::GETTER;
        } else {
            self.inner.to_mut().values.insert(
                PropKey(id),
                (
                    PropFlag::GETTER | PropFlag::THREE,
                    PropCell { getter: getter },
                ),
            );
        }
    }

    pub fn bind_setter(&self, id: u32, setter: JObject) {
        if let Some((flag, cell)) = self.inner.to_mut().values.get_mut(&PropKey(id)) {
            if flag.is_getter() && flag.is_setter() {
                *cell = PropCell {
                    getsetter: (unsafe { cell.getsetter.0 }, setter),
                };
            } else if flag.is_getter() {
                *cell = PropCell {
                    getsetter: (unsafe { cell.getter }, setter),
                };
            } else {
                *cell = PropCell { setter: setter };
            };
            *flag = *flag | PropFlag::SETTER;
        } else {
            self.inner.to_mut().values.insert(
                PropKey(id),
                (
                    PropFlag::SETTER | PropFlag::THREE,
                    PropCell { setter: setter },
                ),
            );
        }
    }

    #[inline]
    pub fn call(
        &self,
        runtime: &Runtime,
        this: JValue,
        stack: *mut JValue,
        argc: u32,
    ) -> (JValue, bool) {
        self.inner.call(runtime, this, stack, argc as usize)
    }

    pub fn keys(&self) -> &'static [u32] {
        let mut v = Vec::with_capacity(self.inner.values.len());
        for i in self.inner.values.keys() {
            v.push(i.0)
        }
        v.leak()
    }

    pub fn deep_clone(&self) -> JObject {
        let obj = JObjectInner::new();
        *obj = self.inner.clone();
        JObject { inner: obj }
    }

    #[inline]
    pub unsafe fn trace(self) {
        if self.inner.flag == GcFlag::Used {
            return;
        }
        self.inner.to_mut().flag = GcFlag::Used;
        for (flag, cell) in self.inner.values.values() {
            if flag.is_getter() && flag.is_setter() {
                let (g, s) = cell.getsetter;
                g.trace();
                s.trace();
            } else if flag.is_getter() {
                cell.getter.trace()
            } else if flag.is_setter() {
                cell.setter.trace()
            } else {
                cell.value.trace()
            }
        }

        if let Some(p) = &self.inner.__proto__{
            p.trace();
        }

        self.inner.wrapped_value.trace();

    }
}

impl From<&'static JObjectInner> for JObject {
    fn from(o: &'static JObjectInner) -> Self {
        Self { inner: o }
    }
}

impl From<&'static mut JObjectInner> for JObject {
    fn from(o: &'static mut JObjectInner) -> Self {
        Self { inner: o }
    }
}

#[derive(Clone)]
#[repr(C)]
pub struct JObjectInner {
    pub(crate) flag: GcFlag,

    pub(crate) values: PropMap,

    pub(crate) __proto__: Option<JObject>,

    pub(crate) wrapped_value: JObjectValue,
}

impl Default for JObjectInner {
    fn default() -> Self {
        JObjectInner {
            flag: GcFlag::Used,
            values: Default::default(),
            __proto__: None,
            wrapped_value: JObjectValue::Empty,
        }
    }
}

impl JObjectInner {
    pub fn new() -> &'static mut JObjectInner {
        let runtime = Runtime::current();
        runtime.allocate_obj()
    }

    pub fn has_owned_property(&self, key: &str) -> bool {
        //let r = hash_(key);
        let runtime = Runtime::current();
        let r = runtime.register_field_name(key);

        self.values.contains_key(&PropKey(r))
    }

    pub fn has_owned_property_static(&self, key: u32) -> bool {
        self.values.contains_key(&PropKey(key))
    }

    pub fn get(&'static self, key: &str, stack: *mut JValue) -> Option<JValue> {
        let (v, err) = self.get_property(key, stack);
        if err {
            None
        } else {
            if v.is_undefined() {
                None
            } else {
                Some(v)
            }
        }
    }

    pub fn get_static(&'static self, key: u32, stack: *mut JValue) -> (JValue, bool) {
        unsafe { self.get_property_static(key, stack) }
    }

    pub fn set_static(
        &'static self,
        key: u32,
        value: JValue,
        stack: *mut JValue,
    ) -> (JValue, bool) {
        unsafe { self.to_mut().set_property_static(key, value, stack) }
    }

    pub fn get_property(&'static self, key: &str, stack: *mut JValue) -> (JValue, bool) {
        if key == "__proto__" {
            if let Some(obj) = self.__proto__ {
                return (JValue::Object(obj), false);
            } else {
                return (JValue::NULL, false);
            }
        }
        //let r = hash_(key);
        let runtime = Runtime::current();
        let r = runtime.register_field_name(key);

        unsafe { self.get_property_static(r, stack) }
    }

    pub unsafe fn get_property_static(
        &'static self,
        key: u32,
        stack: *mut JValue,
    ) -> (JValue, bool) {
        if let Some((f, v)) = self.values.get(&PropKey(key)) {
            if f.is_getter() && f.is_setter() {
                let this = JValue::Object(JObject { inner: self });
                let runtime = Runtime::current();

                let (re, error) = v.getsetter.0.inner.call(&runtime, this, stack, 0);
                if error {
                    return (re, true);
                }
            } else if f.is_getter() {
                let this = JValue::Object(JObject { inner: self });
                let runtime = Runtime::current();

                let (re, error) = v.getter.inner.call(&runtime, this, stack, 0);
                if error {
                    return (re, true);
                }
            } else if f.is_setter() {
                return (JValue::UNDEFINED, false);
            }
            return (v.value, false);
        }
        (JValue::UNDEFINED, false)
    }

    #[inline]
    pub fn insert_property(&mut self, key: &str, value: JValue, flag: PropFlag) {
        //let r = hash_(key);

        match key{
            "__proto__" => {
                if value.is_object() {
                    self.__proto__ = Some(unsafe { value.value.object });
                } else if value.is_null() {
                    self.__proto__ = None;
                }
                return
            },
            
            _ => {

            }
        };

        let runtime = Runtime::current();
        let r = runtime.register_field_name(key);

        self.values
            .insert(PropKey(r), (flag, PropCell { value: value }));
    }

    pub fn insert_property_static(&mut self, key:u32, value: JValue, flag: PropFlag) {
        self.values.insert(PropKey(key), (flag, PropCell{value:value}));
    }

    pub fn set_property(
        &'static mut self,
        key: &str,
        value: JValue,
        stack: *mut JValue,
    ) -> (JValue, bool) {

        match key{
            "__proto__" => {
                if value.is_object() {
                    self.__proto__ = Some(unsafe { value.value.object });
                } else if value.is_null() {
                    self.__proto__ = None;
                }
                return (JValue::UNDEFINED, false);
            },
            
            _ => {

            }
        };

        //let r = hash_(key);
        let runtime = Runtime::current();
        let r = runtime.register_field_name(key);

        unsafe { self.set_property_static(r, value, stack) }
    }

    pub unsafe fn set_property_static(
        &'static mut self,
        key: u32,
        value: JValue,
        stack: *mut JValue,
    ) -> (JValue, bool) {
        let i = (self as *mut Self as *const Self).as_ref().unwrap();

        if let Some((f, v)) = self.values.get_mut(&PropKey(key)) {
            if f.is_setter() && f.is_getter() {
                let this = JValue::Object(JObject { inner: i });
                let runtime = Runtime::current();

                *stack = value;

                let (re, error) = v.getsetter.1.inner.call(&runtime, this, stack, 1);
                if error {
                    return (re, true);
                }
            } else if f.is_setter() {
                let this = JValue::Object(JObject { inner: i });
                let runtime = Runtime::current();

                *stack = value;

                let (re, error) = v.setter.inner.call(&runtime, this, stack, 1);
                if error {
                    return (re, true);
                }
            } else if f.is_getter() {
            } else {
                *v = PropCell { value: value };
            }
        } else {
            self.values
                .insert(PropKey(key), (PropFlag::THREE, PropCell { value: value }));
        }

        (JValue::UNDEFINED, false)
    }

    pub fn remove_key_static(&'static mut self, key: u32) {
        self.values.remove(&PropKey(key));
    }

    pub fn get_own_property_descriptors(&self) -> PropMap {
        let mut h = PropMap::default();

        for (hashing, (flag, value)) in &self.values {
            if !flag.is_getter() && !flag.is_setter() {
                let o = ObjectBuilder::new()
                .field("enumerable", flag.is_enumerable())
                .field("writable", flag.is_writable())
                .field("configurable", flag.is_configurable())
                .field("value", unsafe{value.value})
                .build();

                h.insert(
                    *hashing,
                    (
                        PropFlag::THREE,
                        PropCell {
                            value: JValue::Object(o),
                        },
                    ),
                );
            }
        }
        return h;
    }

    /// this function handles function call and new call
    #[inline]
    pub fn call(
        &'static self,
        runtime: &Runtime,
        this: JValue,
        stack: *mut JValue,
        argc: usize,
    ) -> (JValue, bool) {
        if let Some(func) = self.wrapped_value.function() {
            return func.call(runtime, this, stack, argc)
        } 
        
        if let Some(c) = self.wrapped_value.class(){
            return c.call(runtime, JObject{inner:self}.into(), this, stack, argc);
        }

        return (JValue::Error(Error::CallOnNonFunction), true)
    }

    pub(crate) fn to_mut(&self) -> &mut Self {
        unsafe { (self as *const Self as *mut Self).as_mut().unwrap() }
    }
}

#[derive(Clone)]
pub enum JObjectValue {
    Empty,
    NewTarget,
    Error(Error),

    String(JString),
    Number(f64),
    BigInt(i64),
    Boolean(bool),
    Symbol(JSymbol),

    Array(Vec<(PropFlag, JValue)>),
    ArrayIterator(&'static JObject),
    Function(JSFunctionInstance),
    Generator(Generator),
    Class(JSClassInstance),

    Regex(Arc<RegExp>),
    Promise(Promise),
    Proxy(Proxy),

    Map(HashMap<JValue, JValue>),
    Set(HashMap<JValue, ()>),
    WeakMap(HashMap<u64, JValue>),
    WeakSet(HashMap<u64, ()>),

    /// ArrayBuffers are shared
    ArrayBuffer(Arc<Vec<u8>>),
    DataView(()),

    TypedArray(TypedArray<()>)
}

impl JObjectValue {
    pub fn is_primitive(&self) -> bool {
        match self {
            Self::BigInt(_b) => true,
            Self::Number(_n) => true,
            Self::String(_s) => true,
            Self::Symbol(_s) => true,
            _ => false,
        }
    }

    pub fn is_array(&self) -> bool {
        match self {
            Self::Array(_) => true,
            _ => false,
        }
    }

    pub fn is_function(&self) -> bool {
        match self {
            Self::Function(_f) => true,
            _ => false,
        }
    }

    pub fn array(&self) -> Option<&Vec<(PropFlag, JValue)>> {
        match self {
            Self::Array(a) => Some(a),
            _ => None,
        }
    }

    pub fn number(&self) -> Option<f64> {
        match self {
            Self::Number(f) => Some(*f),
            _ => None,
        }
    }

    pub fn string(&self) -> Option<&JString> {
        match self {
            Self::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn function(&self) -> Option<&JSFunctionInstance> {
        match self {
            Self::Function(f) => Some(f),
            _ => None,
        }
    }

    pub fn class(&self) -> Option<&JSClassInstance> {
        match self{
            Self::Class(c) => Some(c),
            _ => None
        }
    }

    pub fn promise(&self) -> Option<&Promise> {
        match self {
            Self::Promise(p) => Some(p),
            _ => None,
        }
    }

    pub unsafe fn trace(&self){
        match self{
            Self::Array(a) => {
                for (_f, v) in a{
                    v.trace();
                }
            },
            Self::ArrayIterator(a) => {
                a.trace();
            },
            Self::Function(f) => {
                f.trace();
            },
            Self::Generator(_g) => {

            },
            Self::Class(c) => {
                if let Some(f) = &c.constructor_instance{
                    f.trace();
                }
            },
            Self::Map(m) => {
                for (key, v) in m{
                    key.trace();
                    v.trace();
                }
            },
            Self::Proxy(p) => {
                p.handler.trace();
                p.target.trace();
            },
            Self::Set(s) => {
                for (key, _) in s{
                    key.trace();
                }
            },
            Self::WeakMap(m) => {
                for (_, v) in m{
                    v.trace();
                }
            },
            _ => {}
        }
    }
}
