use num_bigint::ToBigInt;
use num_traits::{ToPrimitive, Zero};

use crate::runtime::GcFlag;

#[repr(u8)]
pub enum Sign {
    Plus,
    Minus,
}

#[repr(C)]
pub struct JSBigInt {
    pub flag: GcFlag,
    //pub sign:Sign,
    pub value: num_bigint::BigInt,
}

impl ToString for JSBigInt {
    fn to_string(&self) -> String {
        self.value.to_string()
    }
}

impl JSBigInt {
    pub fn zero() -> Self {
        Self {
            flag: GcFlag::Used,
            //sign:Sign::Plus,
            value: num_bigint::BigInt::zero(),
        }
    }

    pub fn set_value<T>(&mut self, v: T)
    where
        T: ToBigInt,
    {
        self.value = v.to_bigint().unwrap()
    }

    pub fn to_i128(&self) -> i128 {
        self.value.to_i128().unwrap()
    }
}
