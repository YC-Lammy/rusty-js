use std::collections::HashMap;

use super::object::TSObject;

pub struct InterfaceInfo{
    fields:HashMap<u32, usize>
}

// we make sure the size is aligned
#[repr(C)]
pub struct TSInterface{
    ty:*const InterfaceInfo,
    object:TSObject,
    /// each index represents an offset on the TSObject
    mapping:*const [usize],
    length:usize
}