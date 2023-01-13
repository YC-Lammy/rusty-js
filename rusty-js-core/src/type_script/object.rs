use std::collections::HashMap;

use std::sync::Arc;

use super::InterfaceId;
use super::Type;
use crate::runtime::GcFlag;

pub struct ObjectInfo {
    /// a sorted array
    pub properties: Vec<(u32, Type)>,

    /// null or object, the prototype of this type
    pub prototype: Option<Arc<ObjectInfo>>,

    pub implements: Vec<InterfaceId>,

    /// to create this type, simply clone this object
    pub cached_object: Option<TSObject>,
}

impl PartialEq for ObjectInfo{
    fn eq(&self, other: &Self) -> bool {
        self.prototype == other.prototype &&
        self.properties.as_slice() == other.properties.as_slice() &&
        self.implements == other.implements
    }
}

#[derive(Clone, Copy)]
pub struct TSObject {
    /// header is determind by the offset,
    /// a TSObject may be a part of an Object or the object itself
    inner: &'static TSObjectFragment,
}

#[repr(C)]
pub struct TSObjectHeader {
    flag: GcFlag,
    total_length: u32,
}

#[repr(C)]
pub struct TSObjectFragment {
    /// offset from the header
    offset: u32,
    /// length of this fragment
    length: u32,
    /// pointer to the type info
    ty: *const ObjectInfo,

    data: [u64; 0],
}

impl ObjectInfo {
    pub fn property_offset(&self, property:u32) -> Option<usize>{
        let mut i = 0;
        for (p, _t) in &self.properties{
            if *p == property{
                return Some((i) as usize * 8)
            }
            i += 1;
        }
        return None
    }

    pub fn fragment_length(&self) -> usize {
        (self.properties.len() * 8) + std::mem::size_of::<TSObjectFragment>()
    }

    pub fn total_length(&self) -> usize{
        if let Some(p) = &self.prototype{
            return self.fragment_length() + p.total_length()
        } else{
            return self.fragment_length()
        }
        
    }
}

impl TSObject {
    fn get_header(&self) -> &mut TSObjectHeader {
        let offset = self.inner.offset;
        let ptr = self.inner as *const _ as *mut u8;
        unsafe { &mut *(ptr.sub(offset as usize) as *mut TSObjectHeader) }
    }

    pub fn has_parent(&self) -> bool {
        let l = self.get_header().total_length as i32;
        return (l
            - self.inner.offset as i32
            - self.inner.length as i32
            - std::mem::size_of::<TSObjectFragment>() as i32)
            > 0;
    }

    pub fn has_child(&self) -> bool {
        return self.inner.offset as usize != std::mem::size_of::<TSObjectHeader>();
    }
}
