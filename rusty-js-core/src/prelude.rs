use std::ops::{Deref, DerefMut};

use crate::types::JValue;

#[derive(PartialEq, Eq, PartialOrd)]
pub struct BorrowedJSValue{
    value:JValue
}