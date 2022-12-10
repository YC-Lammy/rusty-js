use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::ops::DerefMut;
use std::sync::Arc;

use parking_lot::RwLock;

use crate::error::Error;
use crate::runtime::Runtime;
use crate::runtime::{GcFlag, ModuleId};
use crate::types::JValue;
use crate::{JSBigInt, JSContext};

use crate::utils::nohasher::NoHasherBuilder;
use crate::utils::string_interner::NAMES;

use super::class::JSClassInstance;
use super::function::JSFunctionInstance;
use super::generator::JSGenerator;
use super::object_builder::ObjectBuilder;
use super::promise::Promise;
use super::proxy::Proxy;
use super::regex::RegExp;
use super::strings::JSString;
use super::symbol::JSymbol;
use super::typed_array::TypedArray;

use super::flag::PropFlag;

#[derive(Hash, Clone, Copy, PartialEq, Eq)]
pub struct PropKey(pub(crate) u32);

pub trait ToProperyKey {
    fn to_key(&self, runtime: &Runtime) -> PropKey;
}

impl ToProperyKey for PropKey {
    fn to_key(&self, _runtime: &Runtime) -> PropKey {
        return *self;
    }
}

impl ToProperyKey for JValue {
    fn to_key(&self, runtime: &Runtime) -> PropKey {
        if let Some(s) = self.as_string() {
            let id = runtime.register_field_name(s.as_ref());
            return PropKey(id);
        } else {
            let s = self.to_string();
            let id = runtime.register_field_name(&s);
            return PropKey(id);
        }
    }
}

impl<S> ToProperyKey for S
where
    S: AsRef<str>,
{
    fn to_key(&self, runtime: &Runtime) -> PropKey {
        let id = runtime.register_field_name(self.as_ref());
        PropKey(id)
    }
}

#[derive(Clone, Copy)]
pub struct PropCell {
    pub flag: PropFlag,
    /// value acts as getter when flag has getter
    pub value: JValue,
    pub setter: JValue,
}

pub type PropMap = HashMap<PropKey, PropCell, NoHasherBuilder>;

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

pub trait CustomObject: std::any::Any {
    fn get(&mut self, name: &str) -> Option<JValue>;
    fn set(&mut self, name: &str, value: &JValue) -> bool;
    fn own_properties(&self) -> &[&str];

    #[allow(unused_variables)]
    fn call(&mut self, this: &JValue, args: &[JValue]) -> Result<JValue, JValue> {
        return Err(JValue::from(Error::CallOnNonFunction));
    }
}

impl dyn CustomObject {
    pub fn downcast_ref<T: 'static>(&self) -> Option<&T> {
        if self.type_id() == std::any::TypeId::of::<T>() {
            Some(unsafe { &*(self as *const dyn CustomObject as *const T) })
        } else {
            None
        }
    }
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

impl JObject {
    pub fn new() -> Self {
        let rt = Runtime::current();
        rt.create_object()
    }

    pub fn new_target() -> Self {
        let obj = Self::new();
        obj.inner.to_mut().wrapped_value = JObjectValue::NewTarget;
        return obj;
    }

    pub fn is_extensible(&self) -> bool {
        self.inner.extensible
    }

    pub fn is_new_target(&self) -> bool {
        match &self.inner.wrapped_value {
            JObjectValue::NewTarget => true,
            _ => false,
        }
    }

    pub fn from_custom_object(obj: Arc<dyn CustomObject>) -> JObject {
        let rt = Runtime::current();
        let inner = rt.allocate_obj();
        inner.__proto__ = Some(rt.prototypes.array);
        inner.wrapped_value = JObjectValue::CustomObject(obj);

        return JObject { inner: inner };
    }

    pub fn as_custom_object<'a>(&'a self) -> Option<&'a Arc<dyn CustomObject>> {
        match &self.inner.wrapped_value {
            JObjectValue::CustomObject(c) => Some(c),
            _ => None,
        }
    }

    pub fn bind_custom_object(&self, obj: Arc<dyn CustomObject>) {
        self.inner.to_mut().wrapped_value = JObjectValue::CustomObject(obj)
    }

    pub fn is_custom_object(&self) -> bool {
        self.as_custom_object().is_some()
    }

    pub fn array() -> Self {
        let rt = Runtime::current();
        let inner = rt.allocate_obj();
        inner.__proto__ = Some(rt.prototypes.array);
        inner.wrapped_value = JObjectValue::Array(Arc::new(Vec::new()));
        let obj = JObject { inner };
        obj.insert_property(
            NAMES["length"],
            JValue::create_number(0.0),
            PropFlag::WRITABLE,
        );

        return JObject { inner: inner };
    }

    pub fn with_array(a: Vec<(PropFlag, JValue)>) -> Self {
        let len = a.len();
        let rt = Runtime::current();
        let inner = rt.allocate_obj();
        inner.__proto__ = Some(rt.prototypes.array);
        inner.wrapped_value = JObjectValue::Array(Arc::new(a));
        let obj = JObject { inner: inner };
        obj.insert_property(
            NAMES["length"],
            JValue::create_number(len as f64),
            PropFlag::WRITABLE,
        );

        return JObject { inner: inner };
    }

    pub fn new_map() -> Self {
        Self::with_value(JObjectValue::Map(Default::default()))
    }

    pub fn new_set() -> Self {
        Self::with_value(JObjectValue::Set(Default::default()))
    }

    pub fn weak_set() -> Self {
        Self::with_value(JObjectValue::WeakSet(Default::default()))
    }

    pub fn weak_map() -> Self {
        Self::with_value(JObjectValue::WeakMap(Default::default()))
    }

    /// todo: use f.create_object instead
    pub fn with_function(f: JSFunctionInstance) -> Self {
        f.create_object()
    }

    pub fn with_error(e: Error) -> Self {
        let rt = Runtime::current();
        let inner = rt.allocate_obj();
        inner.__proto__ = Some(rt.prototypes.error);
        inner.wrapped_value = JObjectValue::Error(Box::new(e));

        return JObject { inner: inner };
    }

    pub fn with_promise(p: Promise) -> Self {
        let rt = Runtime::current();
        let inner = rt.allocate_obj();
        inner.__proto__ = Some(rt.prototypes.promise);
        inner.wrapped_value = JObjectValue::Promise(Box::new(p));

        return JObject { inner: inner };
    }

    pub fn with_regex(r: Box<RegExp>) -> Self {
        let rt = Runtime::current();
        let inner = rt.allocate_obj();
        inner.__proto__ = Some(rt.prototypes.regex);
        inner.wrapped_value = JObjectValue::Regex(r);

        return JObject { inner: inner };
    }

    pub fn with_number(n: f64) -> Self {
        let rt = Runtime::current();
        let inner = rt.allocate_obj();
        inner.__proto__ = Some(rt.prototypes.number);
        inner.wrapped_value = JObjectValue::Number(n);

        return JObject { inner: inner };
    }

    pub fn with_module(id: ModuleId) -> Self {
        Self::with_value(JObjectValue::Module(id))
    }

    /// with value is private
    fn with_value(value: JObjectValue) -> Self {
        let obj = JObject::new();
        obj.inner.to_mut().wrapped_value = value;
        obj
    }

    pub fn set_inner(&self, value: JObjectValue) {
        self.inner.to_mut().wrapped_value = value;
    }

    pub fn as_array<'a>(&'a self) -> Option<&'a mut Vec<(PropFlag, JValue)>> {
        match &mut self.inner.to_mut().wrapped_value {
            JObjectValue::Array(a) => {
                Some(unsafe { &mut *(a.as_ref() as *const Vec<_> as *mut Vec<_>) })
            }
            _ => None,
        }
    }

    pub fn as_arc_array<'a>(&'a self) -> Option<&'a mut Arc<Vec<(PropFlag, JValue)>>> {
        match &mut self.inner.to_mut().wrapped_value {
            JObjectValue::Array(a) => Some(a),
            _ => None,
        }
    }

    pub fn is_array(&self) -> bool {
        self.inner.wrapped_value.is_array()
    }

    pub fn as_function_instance<'a>(&'a self) -> Option<&'a JSFunctionInstance> {
        match &self.inner.wrapped_value {
            JObjectValue::Function(f) => Some(f.as_ref()),
            _ => None,
        }
    }

    pub fn is_function_instance(&self) -> bool {
        self.as_function_instance().is_some()
    }

    pub fn as_native_function(
        &self,
    ) -> Option<&Arc<RwLock<dyn Fn(JSContext, JValue, &[JValue]) -> Result<JValue, JValue>>>> {
        match &self.inner.wrapped_value {
            JObjectValue::NativeFunction(f) => Some(f),
            _ => None,
        }
    }

    pub fn is_native_function(&self) -> bool {
        self.as_native_function().is_some()
    }

    pub fn as_regexp(&self) -> Option<&mut RegExp> {
        match &mut self.inner.to_mut().wrapped_value {
            JObjectValue::Regex(r) => Some(r),
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

    pub fn is_error(&self) -> bool {
        self.as_error().is_some()
    }

    pub fn as_number(&self) -> Option<f64> {
        match &self.inner.wrapped_value {
            JObjectValue::Number(n) => Some(*n),
            _ => None,
        }
    }

    pub fn is_number(&self) -> bool {
        self.as_number().is_some()
    }

    pub fn as_class<'a>(&'a self) -> Option<&'a JSClassInstance> {
        match &self.inner.wrapped_value {
            JObjectValue::Class(c) => Some(c.as_ref()),
            _ => None,
        }
    }
    pub fn is_class(&self) -> bool {
        self.as_class().is_some()
    }

    pub fn is_primitive(&self) -> bool {
        self.inner.wrapped_value.is_primitive()
    }

    pub fn to_primitive(&self) -> Option<JValue> {
        None
    }

    pub fn has_owned_property(&self, key: PropKey) -> bool {
        self.inner.values.contains_key(&key)
    }

    pub fn has_property(&self, key: PropKey) -> bool {
        if self.has_owned_property(key) {
            return true;
        } else {
            todo!()
        }
    }

    pub fn has_property_static(&self, key: u32) -> bool {
        if self.inner.has_owned_property_static(key) {
            return true;
        } else {
            todo!()
        }
    }

    pub fn has_owned_property_static(&self, key: u32) -> bool {
        self.inner.has_owned_property_static(key)
    }

    pub fn has_owned_setter(&self, key: PropKey) -> bool {
        if let Some(cell) = self.inner.values.get(&key) {
            if cell.flag.is_setter() {
                return true;
            }
        }
        return false;
    }

    pub fn remove_property(&self, key: PropKey) {
        self.inner.to_mut().values.remove(&key);
    }

    pub fn get_property<K>(&self, key: K, ctx: JSContext) -> Result<JValue, JValue>
    where
        K: ToProperyKey,
    {
        let key = key.to_key(&ctx.runtime);

        if key == NAMES["__proto__"] {
            if let Some(obj) = self.inner.__proto__ {
                return Ok(JValue::create_object(obj));
            } else {
                return Ok(JValue::NULL);
            }
        }

        match self.inner.to_mut().wrapped_value.get_property(key.0) {
            Some(v) => return Ok(v),
            None => {}
        };

        if let Some(cell) = self.inner.values.get(&key) {
            if !cell.flag.is_getter() && !cell.flag.is_setter() {
                // a data property
                return Ok(cell.value);
            }

            if cell.flag.is_getter() {
                let this = JValue::create_object(*self);

                return cell.value.call(this, &[], ctx);
            } else if cell.flag.is_setter() {
                return Ok(JValue::UNDEFINED);
            }
        };

        let mut proto = self.inner.__proto__;
        // loop through the prototypes to find the property

        loop {
            if let Some(p) = proto {
                if let Some(cell) = p.inner.values.get(&key) {
                    if !cell.flag.is_getter() && !cell.flag.is_setter() {
                        // a data property
                        return Ok(cell.value);
                    }

                    if cell.flag.is_getter() {
                        let this = JValue::create_object(p);

                        return cell.value.call(this, &[], ctx);
                    } else if cell.flag.is_setter() {
                        return Ok(JValue::UNDEFINED);
                    }
                } else {
                    proto = p.inner.__proto__;
                };
            } else {
                break;
            }
        }
        Ok(JValue::UNDEFINED)
    }

    #[inline]
    pub fn insert_property(&self, key: PropKey, value: JValue, flag: PropFlag) {
        if key == NAMES["__proto__"] {
            if let Some(obj) = value.as_object() {
                self.inner.to_mut().__proto__ = Some(obj);
            }
        }
        self.inner.to_mut().values.insert(
            key,
            PropCell {
                flag,
                value,
                setter: JValue::UNDEFINED,
            },
        );
    }

    pub fn insert_property_builtin(&self, key: PropKey, value: JValue) {
        self.insert_property(key, value, PropFlag::BUILTIN)
    }

    pub fn set_property<K: ToProperyKey>(
        &self,
        key: K,
        value: JValue,
        ctx: JSContext,
    ) -> Result<(), JValue> {
        let key = key.to_key(ctx.runtime);
        if key == NAMES["__proto__"] {
            if let Some(obj) = value.as_object() {
                self.inner.to_mut().__proto__ = Some(obj);
            } else if value.is_null() {
                self.inner.to_mut().__proto__ = None;
            }
            return Ok(());
        };

        if let Some(cell) = self.inner.to_mut().values.get_mut(&key) {
            if cell.flag.is_setter() {
                let this = JValue::create_object(*self);

                cell.setter.call(this, &[value], ctx)?;
                return Ok(());
            } else if cell.flag.is_getter() {
                // do nothing
            } else {
                if cell.flag.is_writable() {
                    cell.value = value;
                }
            };
        } else {
            // loop through the parents and find a setter
            let mut parent = self.inner.__proto__;
            loop {
                if let Some(p) = parent {
                    if p.has_owned_setter(key) {
                        return p.set_property(key, value, ctx);
                    } else {
                        parent = p.inner.__proto__;
                    };
                } else {
                    break;
                }
            }

            // return error if not extendable
            if !self.inner.extensible {
                let n = ctx.runtime.get_field_name(key.0);
                return Err(JValue::from(Error::TypeError(format!(
                    "Cannot add property {}, object is not extensible",
                    n
                ))));
            };

            self.inner.to_mut().values.insert(
                key,
                PropCell {
                    flag: PropFlag::THREE,
                    value: value,
                    setter: JValue::UNDEFINED,
                },
            );
        }

        Ok(())
    }

    pub fn bind_getter(&self, key: PropKey, getter: JObject) {
        if let Some(cell) = self.inner.to_mut().values.get_mut(&key) {
            if cell.flag.is_getter() || cell.flag.is_setter() {
                cell.value = getter.into();
            } else {
                cell.value = getter.into();
                cell.flag = cell.flag | PropFlag::GETTER;
            };
        } else {
            self.inner.to_mut().values.insert(
                key,
                PropCell {
                    flag: PropFlag::THREE | PropFlag::GETTER,
                    value: getter.into(),
                    setter: JValue::UNDEFINED,
                },
            );
        }
    }

    pub fn bind_setter(&self, key: PropKey, setter: JObject) {
        if let Some(cell) = self.inner.to_mut().values.get_mut(&key) {
            if cell.flag.is_getter() || cell.flag.is_setter() {
                cell.setter = setter.into();
            } else {
                cell.setter = setter.into();
                cell.flag = cell.flag | PropFlag::SETTER;
            };
        } else {
            self.inner.to_mut().values.insert(
                key,
                PropCell {
                    flag: PropFlag::THREE | PropFlag::SETTER,
                    value: JValue::UNDEFINED,
                    setter: setter.into(),
                },
            );
        }
    }

    #[inline]
    pub fn call(
        &self,
        runtime: &Runtime,
        this: JValue,
        stack: *mut JValue,
        argc: usize,
    ) -> (JValue, bool) {
        if let Some(func) = self.inner.wrapped_value.function() {
            return func.clone().call(runtime, this, stack, argc);
        }

        if let Some(c) = self.inner.wrapped_value.class() {
            return c.call(runtime, (*self).into(), this, stack, argc);
        }

        if let Some(f) = self.as_native_function() {
            let mut guard = f.write();
            let args = unsafe { std::slice::from_raw_parts(stack, argc) };
            let re = (guard.deref_mut())(
                JSContext {
                    stack: unsafe { stack.add(argc) },
                    runtime,
                },
                this,
                args,
            );
            match re {
                Ok(v) => return (v, false),
                Err(e) => return (e, true),
            }
        };

        return (JValue::from(Error::CallOnNonFunction), true);
    }

    pub fn keys(&self) -> &'static [u32] {
        let mut v = Vec::with_capacity(self.inner.values.len());
        for i in self.inner.values.keys() {
            v.push(i.0)
        }
        v.leak()
    }

    pub fn deep_clone(&self) -> JObject {
        let obj = JObject::new();
        *obj.inner.to_mut() = self.inner.clone();
        obj
    }

    #[inline]
    pub unsafe fn trace(self) {
        if self.inner.flag == GcFlag::Garbage {
            return;
        }
        if self.inner.flag == GcFlag::Used {
            return;
        }
        self.inner.to_mut().flag = GcFlag::Used;
        for cell in self.inner.values.values() {
            cell.value.trace();
            cell.setter.trace();
        }

        if let Some(p) = &self.inner.__proto__ {
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
    // 1 byte
    pub(crate) flag: GcFlag,

    // this field serves as prototype of the object and the chained list in allocater
    pub(crate) __proto__: Option<JObject>,

    // 1 byte
    pub(crate) extensible: bool,

    // 48 bytes
    pub(crate) values: PropMap,

    pub(crate) wrapped_value: JObjectValue,
}

impl Default for JObjectInner {
    fn default() -> Self {
        JObjectInner {
            flag: GcFlag::Used,
            //next_obj: 0 as _,
            extensible: true,
            values: Default::default(),
            __proto__: None,
            wrapped_value: JObjectValue::Empty,
        }
    }
}

impl JObjectInner {

    pub fn has_owned_property_static(&self, key: u32) -> bool {
        self.values.contains_key(&PropKey(key))
    }

    pub fn remove_key_static(&'static mut self, key: u32) {
        self.values.remove(&PropKey(key));
    }

    pub(crate) fn to_mut(&self) -> &mut Self {
        unsafe { &mut *(self as *const Self as *mut Self) }
    }
}

#[derive(Clone)]
pub enum JObjectValue {
    Empty,
    NewTarget,

    Module(ModuleId),

    Error(Box<Error>),

    String(JSString),
    Number(f64),
    BigInt(&'static JSBigInt),
    Boolean(bool),
    Symbol(JSymbol),

    Array(Arc<Vec<(PropFlag, JValue)>>),
    ArrayIterator(&'static JObject),
    Function(Arc<JSFunctionInstance>),
    NativeFunction(Arc<RwLock<dyn Fn(JSContext, JValue, &[JValue]) -> Result<JValue, JValue>>>),
    Generator(Box<JSGenerator>),
    Class(Arc<JSClassInstance>),

    Regex(Box<RegExp>),
    Promise(Box<Promise>),
    Proxy(Box<Proxy>),

    Map(Box<HashMap<JValue, JValue>>),
    Set(Box<HashMap<JValue, ()>>),
    WeakMap(Box<HashMap<u64, JValue>>),
    WeakSet(Box<HashMap<u64, ()>>),

    /// ArrayBuffers are shared
    ArrayBuffer(Arc<Vec<u8>>),
    DataView(()),

    TypedArray(Box<TypedArray<()>>),

    CustomObject(Arc<dyn CustomObject>),
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

    pub(crate) fn arc_array(&self) -> Option<&Arc<Vec<(PropFlag, JValue)>>> {
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

    pub fn string(&self) -> Option<&JSString> {
        match self {
            Self::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn function(&self) -> Option<&Arc<JSFunctionInstance>> {
        match self {
            Self::Function(f) => Some(f),
            _ => None,
        }
    }

    pub fn class(&self) -> Option<&JSClassInstance> {
        match self {
            Self::Class(c) => Some(c),
            _ => None,
        }
    }

    pub fn promise(&self) -> Option<&Promise> {
        match self {
            Self::Promise(p) => Some(p),
            _ => None,
        }
    }

    pub fn get_property(&mut self, key: u32) -> Option<JValue> {
        match self {
            Self::Array(a) => {
                macro_rules! match_name {
                    ($idx:tt) => {
                        if key == NAMES[stringify!($idx)].0 {
                            return a.get($idx).and_then(|v| Some(v.1));
                        }
                    };
                }

                match_name!(0);
                match_name!(1);
                match_name!(2);
                match_name!(3);
                match_name!(4);
                match_name!(5);
                match_name!(6);
                match_name!(7);
                match_name!(8);
                match_name!(9);
                match_name!(10);

                let rt = Runtime::current();
                let key = rt.get_field_name(key);

                if let Ok(v) = fast_float::parse::<f64, _>(key) {
                    return a.get(v as usize).and_then(|v| Some(v.1));
                }
            }
            _ => {}
        };
        None
    }

    #[inline]
    pub unsafe fn trace(&self) {
        match self {
            Self::Array(a) => {
                for (_f, v) in a.as_ref() {
                    v.trace();
                }
            }
            Self::ArrayIterator(a) => {
                a.trace();
            }
            Self::Function(f) => {
                f.trace();
            }
            Self::Generator(_g) => {}
            Self::Class(c) => {
                if let Some(f) = &c.constructor_instance {
                    f.trace();
                }

                if let Some(v) = &c.super_ {
                    v.trace();
                }
            }
            Self::Map(m) => {
                for (key, v) in m.iter() {
                    key.trace();
                    v.trace();
                }
            }
            Self::Proxy(p) => {
                p.handler.trace();
                p.target.trace();
            }
            Self::Promise(p) => {
                match p.as_ref() {
                    Promise::Fulfilled(f) => {
                        f.trace();
                    }
                    Promise::Rejected(r) => {
                        r.trace();
                    }
                    Promise::Pending { id: _ } => {}
                    Promise::ForeverPending => {}
                };
            }
            Self::Set(s) => {
                for (key, _) in s.iter() {
                    key.trace();
                }
            }
            Self::String(s) => {
                s.trace();
            }
            Self::WeakMap(m) => {
                for (_, v) in m.iter() {
                    v.trace();
                }
            }
            _ => {}
        }
    }
}

impl Default for JObjectValue {
    fn default() -> Self {
        JObjectValue::Empty
    }
}
