pub mod class;
pub mod function;
pub mod object;
pub mod object_builder;
pub mod custom_object;
pub mod promise;
pub mod prop;
pub mod proxy;
pub mod regex;
pub mod strings;
pub mod symbol;
pub mod typed_array;
pub mod generator;

use object::JObject;

use crate::Runtime;

pub struct BuiltinPrototypes{
    pub function:JObject,
    pub boolean:JObject,
    pub symbol:JObject,
    pub error:JObject,
    pub number:JObject,
    pub bigint:JObject,
    pub date:JObject,
    pub string:JObject,
    pub regex:JObject,
    pub array:JObject,
    pub typed_array:JObject,
    pub map:JObject,
    pub set:JObject,
    pub weakmap:JObject,
    pub weakset:JObject,
    pub array_buffer:JObject,
    pub data_view:JObject,
    pub weak_ref:JObject,
    pub finalization_registry:JObject,
    
}

impl BuiltinPrototypes{
    
    pub fn zero() -> Self{
        unsafe{
            #[allow(invalid_value)]
            let o = std::mem::MaybeUninit::uninit().assume_init();
            Self { 
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
                map: o, 
                set: o, 
                weakmap: o, 
                weakset: o, 
                array_buffer: o, 
                data_view: o, 
                weak_ref: o, 
                finalization_registry: o
            }
        }
    }

    pub fn init(&mut self, rt:&mut Runtime){
        *self = Self { 
            function: rt.allocate_obj().into(), 
            boolean: rt.allocate_obj().into(), 
            symbol: rt.allocate_obj().into(), 
            error: rt.allocate_obj().into(), 
            number: rt.allocate_obj().into(), 
            bigint: rt.allocate_obj().into(), 
            date: rt.allocate_obj().into(), 
            string: rt.allocate_obj().into(), 
            regex: rt.allocate_obj().into(), 
            array: rt.allocate_obj().into(), 
            typed_array: rt.allocate_obj().into(), 
            map: rt.allocate_obj().into(), 
            set: rt.allocate_obj().into(), 
            weakmap: rt.allocate_obj().into(), 
            weakset: rt.allocate_obj().into(), 
            array_buffer: rt.allocate_obj().into(), 
            data_view: rt.allocate_obj().into(), 
            weak_ref: rt.allocate_obj().into(), 
            finalization_registry: rt.allocate_obj().into()
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