pub mod bigint;
pub mod class;
pub mod custom_object;
pub mod object_property;
pub mod function;
pub mod generator;
pub mod object;
pub mod promise;
pub mod proxy;
pub mod regex;
pub mod strings;
pub mod symbol;
pub mod typed_array;

pub use bigint::JSBigInt;
pub use function::{JSContext, JSFunction, JSFunctionInstance};
pub use object::JObject;

use crate::Runtime;

pub struct BuiltinPrototypes {
    pub object: JObject,
    pub function: JObject,
    pub boolean: JObject,
    pub symbol: JObject,
    pub error: JObject,
    pub number: JObject,
    pub bigint: JObject,
    pub date: JObject,
    pub string: JObject,
    pub regex: JObject,
    pub array: JObject,
    pub typed_array: JObject,
    pub promise: JObject,
    pub map: JObject,
    pub set: JObject,
    pub weakmap: JObject,
    pub weakset: JObject,
    pub array_buffer: JObject,
    pub data_view: JObject,
    pub weak_ref: JObject,
    pub finalization_registry: JObject,
}

impl BuiltinPrototypes {
    pub fn zero() -> Self {
        unsafe {
            #[allow(invalid_value)]
            let o = std::mem::MaybeUninit::uninit().assume_init();
            Self {
                object: o,
                function: o,
                boolean: o,
                symbol: o,
                error: o,
                number: o,
                bigint: o,
                date: o,
                string: o,
                regex: o,
                array: o,
                typed_array: o,
                promise: o,
                map: o,
                set: o,
                weakmap: o,
                weakset: o,
                array_buffer: o,
                data_view: o,
                weak_ref: o,
                finalization_registry: o,
            }
        }
    }

    pub fn init(&mut self, rt: &mut Runtime) {
        *self = Self {
            object: rt.allocate_obj().into(),
            function: rt.create_object().into(),
            boolean: rt.create_object().into(),
            symbol: rt.create_object().into(),
            error: rt.create_object().into(),
            number: rt.create_object().into(),
            bigint: rt.create_object().into(),
            date: rt.create_object().into(),
            string: rt.create_object().into(),
            regex: rt.create_object().into(),
            array: rt.create_object().into(),
            typed_array: rt.create_object().into(),
            promise: rt.create_object().into(),
            map: rt.create_object().into(),
            set: rt.create_object().into(),
            weakmap: rt.create_object().into(),
            weakset: rt.create_object().into(),
            array_buffer: rt.create_object().into(),
            data_view: rt.create_object().into(),
            weak_ref: rt.create_object().into(),
            finalization_registry: rt.create_object().into(),
        };
    }

    #[inline]
    pub unsafe fn trace(&self) {
        self.array.trace();
        self.array_buffer.trace();
        self.bigint.trace();
        self.boolean.trace();
        self.data_view.trace();
        self.date.trace();
        self.error.trace();
        self.finalization_registry.trace();
        self.function.trace();
        self.map.trace();
        self.number.trace();
        self.regex.trace();
        self.set.trace();
        self.string.trace();
        self.symbol.trace();
        self.typed_array.trace();
        self.weak_ref.trace();
        self.weakmap.trace();
        self.weakset.trace();
    }
}
