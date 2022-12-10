use std::collections::HashMap;

use super::InterfaceId;
use super::ObjectId;
use super::Type;
use crate::runtime::GcFlag;

pub struct ObjectInfo {
    pub property_names: HashMap<String, usize>,
    pub properties: Vec<Type>,

    /// null or object, the prototype of this type
    pub prototype: Option<*const ObjectInfo>,
    pub impls: HashMap<InterfaceId, &'static [usize]>,

    /// to create this type, simply clone this object
    pub cached_object: Option<TSObject>,
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
    /// total length including header and fragments
    fn total_length(&self) -> u32 {
        let mut p = std::mem::size_of::<TSObjectHeader>();
        p += self.fragment_length();

        let mut proto = self.prototype;
        while let Some(o) = proto {
            unsafe {
                let o = o.as_ref().unwrap();
                p += o.fragment_length();
                proto = o.prototype;
            }
        }
        return p as u32;
    }

    fn fragment_length(&self) -> usize {
        (self.properties.len() * 8) + std::mem::size_of::<TSObjectFragment>()
    }

    pub fn creat_object(&self) -> TSObject {
        unsafe {
            let total_len = self.total_length();

            let b = Self::alloc_zero(total_len as usize) as *mut TSObjectHeader;
            b.write(TSObjectHeader {
                flag: GcFlag::Used,
                total_length: total_len as u32,
            });

            let first_fragment = b.add(1) as *mut TSObjectFragment;
            self.init_fragment(first_fragment);

            return TSObject {
                inner: first_fragment.as_ref().unwrap(),
            };
        }
    }

    unsafe fn init_fragment(&self, fragment: *mut TSObjectFragment) {
        fragment.write(TSObjectFragment {
            offset: std::mem::size_of::<TSObjectHeader>() as u32,
            length: self.properties.len() as u32 * 8,
            ty: self as *const Self,
            data: [0; 0],
        });
    }

    fn alloc_zero(size: usize) -> *mut u8 {
        todo!()
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
