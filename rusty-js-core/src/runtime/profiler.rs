use std::{alloc::Layout};

use crate::{JValue, types::JSType};


#[repr(C)]
pub struct Profiler {
    pub current: *mut u16,
    pub results: *mut JSType,
    pub len: usize,
}

unsafe impl Sync for Profiler {}
unsafe impl Send for Profiler {}

impl Profiler {
    pub fn new(size:usize) -> Self{
        let (c, r) = unsafe{
            let c = std::alloc::alloc_zeroed(Layout::array::<u16>(size).unwrap()) as *mut u16;
            let r = std::alloc::alloc_zeroed(Layout::array::<JSType>(size).unwrap()) as *mut JSType;

            (c, r)
        };

        Self { current: c, results: r , len: size}
    }

    pub fn finish(&self){
        unsafe{
            let slice = std::slice::from_raw_parts_mut(self.current, self.len);
            let results = std::slice::from_raw_parts_mut(self.results, self.len);

            for i in 0..slice.len(){
                let a = *slice.get_unchecked(i);

                if a == (JValue::INT_TAG >> 48) as u16{
                    *results.get_unchecked_mut(i) |= JSType::Number;

                } else if a == (JValue::NULL_TAG >> 48) as u16{
                    *results.get_unchecked_mut(i) |= JSType::Null;

                } else if a == (JValue::TRUE_TAG >> 48) as u16{
                    *results.get_unchecked_mut(i) |= JSType::Boolean;

                } else if a == (JValue::FALSE_TAG >> 48) as u16{
                    *results.get_unchecked_mut(i) |= JSType::Boolean;

                } else if a == (JValue::BIGINT_TAG >> 48) as u16{
                    *results.get_unchecked_mut(i) |= JSType::Bigint;

                } else if a == (JValue::OBJECT_TAG >> 48) as u16{
                    *results.get_unchecked_mut(i) |= JSType::Object;

                } else if a == (JValue::STRING_TAG >> 48) as u16{
                    *results.get_unchecked_mut(i) |= JSType::String;

                } else if a == (JValue::SYMBOL_TAG >> 48) as u16{
                    *results.get_unchecked_mut(i) |= JSType::Symbol;

                } else if a == (JValue::UNDEFINED_TAG >> 48) as u16{
                    *results.get_unchecked_mut(i) |= JSType::Undefined;

                } else{
                    *results.get_unchecked_mut(i) |= JSType::Number;

                }
            }
        }
    }
}

impl Drop for Profiler{
    fn drop(&mut self) {
        unsafe{
            //let results = std::slice::from_raw_parts_mut(self.results, self.len);
            //println!("{:?}", results);
            std::alloc::dealloc(self.current as _, Layout::array::<u16>(self.len).unwrap());
            std::alloc::dealloc(self.results as _, Layout::array::<JSType>(self.len).unwrap());
        }
    }
}