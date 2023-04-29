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

#[derive(Clone, Copy)]
pub struct TSObject {
    /// header is determind by the offset,
    /// a TSObject may be a part of an Object or the object itself
    inner: &'static TSObjectFragment,
}

#[repr(C)]
pub struct TSObjectFragment {
    flag: GcFlag,
    total_length: u32,
    /// offset from the top child
    offset: u32,
    /// length of this fragment
    length: u16,
    /// pointer to the type info
    ty: *const ObjectInfo,

    data: [u64; 0],
}


impl TSObject {
    pub fn has_parent(&self) -> bool {
        self.inner.total_length != self.inner.length as u32
    }

    pub fn has_child(&self) -> bool {
        return self.inner.offset as usize != 0;
    }

    pub fn get_parent(&self) -> Option<TSObject>{
        if self.has_parent(){
            let offset = self.inner.length as usize;
            let ptr = unsafe{((self.inner as *const _ as *const u8).add(offset) as *const TSObjectFragment).as_ref().unwrap()};
            return Some(TSObject { inner: ptr })
        } else{
            return None
        }
    }

    pub fn get_child(&self) -> Option<TSObject>{
        if self.has_child(){
            let offset = self.inner.offset as usize;
            let ptr = unsafe{((self.inner as *const _ as *const u8).sub(offset) as *const TSObjectFragment).as_ref().unwrap()};
            return Some(TSObject { inner: ptr })
        } else{
            return None
        }
    }
}