use std::{alloc::Layout, marker::PhantomData};

use crate::bultins::strings::JStringHeader;

use super::GcFlag;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct LinkNode<const SIZE: usize> {
    flag: GcFlag,
    next: *mut LinkNode<SIZE>,
    mark: PhantomData<[(); SIZE]>,
}

pub struct StringAllocator {
    pages: Vec<&'static mut [u8; 4096]>,

    s64: *mut LinkNode<64>,
    s128: *mut LinkNode<128>,
    s256: *mut LinkNode<256>,
    s512: *mut LinkNode<512>,
    s1024: *mut LinkNode<1024>,
    s2048: *mut LinkNode<2048>,

    sys: Vec<*mut JStringHeader>,
}

impl StringAllocator {
    pub fn new() -> Self {
        Self {
            pages: Vec::new(),
            s64: std::ptr::null_mut(),
            s128: std::ptr::null_mut(),
            s256: std::ptr::null_mut(),
            s512: std::ptr::null_mut(),
            s1024: std::ptr::null_mut(),
            s2048: std::ptr::null_mut(),
            sys: Vec::new(),
        }
    }
    pub(crate) fn allocate(&mut self, size: usize) -> *mut JStringHeader {
        let size = std::mem::size_of::<JStringHeader>() + size;

        (if size <= 8 {
            LinkNode::allocate(&mut self.s64, self) as *mut u8
        } else if size <= 16 {
            LinkNode::allocate(&mut self.s128, self) as *mut u8
        } else if size <= 32 {
            LinkNode::allocate(&mut self.s256, self) as *mut u8
        } else if size <= 64 {
            LinkNode::allocate(&mut self.s512, self) as *mut u8
        } else if size <= 128 {
            LinkNode::allocate(&mut self.s1024, self) as *mut u8
        } else if size <= 256 {
            LinkNode::allocate(&mut self.s2048, self) as *mut u8
        } else {
            let v = unsafe { std::alloc::alloc(Layout::array::<u8>(size).unwrap()) };
            self.sys.push(v as _);
            v
        }) as *mut JStringHeader
    }
}

impl<const SIZE: usize> LinkNode<SIZE> {
    fn allocate(this: *mut *mut Self, alloc: &mut StringAllocator) -> *mut Self {
        unsafe {
            if (*this).is_null() {
                Self::add_page(alloc);
            };

            let ptr = *this;
            *this = (*ptr).next;
            ptr
        }
    }

    unsafe fn add_page(alloc: &mut StringAllocator) {
        let page = std::alloc::alloc(Layout::new::<[u8; 4096]>()) as *mut [u8; 4096];

        let p = page.as_mut().unwrap();
        alloc.pages.push(p);

        let pages = std::slice::from_raw_parts_mut(page as *mut [u8; SIZE], 4096 / SIZE);

        for i in pages {
            if SIZE == 64 {
                let node = i as *mut _ as *mut _;
                *node = LinkNode {
                    flag: GcFlag::Garbage,
                    next: alloc.s64,
                    mark: PhantomData,
                };
                alloc.s64 = node;
            } else if SIZE == 128 {
                let node = i as *mut _ as *mut _;
                *node = LinkNode {
                    flag: GcFlag::Garbage,
                    next: alloc.s128,
                    mark: PhantomData,
                };
                alloc.s128 = node;
            } else if SIZE == 256 {
                let node = i as *mut _ as *mut _;
                *node = LinkNode {
                    flag: GcFlag::Garbage,
                    next: alloc.s256,
                    mark: PhantomData,
                };
                alloc.s256 = node;
            } else if SIZE == 512 {
                let node = i as *mut _ as *mut _;
                *node = LinkNode {
                    flag: GcFlag::Garbage,
                    next: alloc.s512,
                    mark: PhantomData,
                };
                alloc.s512 = node;
            } else if SIZE == 1024 {
                let node = i as *mut _ as *mut _;
                *node = LinkNode {
                    flag: GcFlag::Garbage,
                    next: alloc.s1024,
                    mark: PhantomData,
                };
                alloc.s1024 = node;
            } else if SIZE == 2048 {
                let node = i as *mut _ as *mut _;
                *node = LinkNode {
                    flag: GcFlag::Garbage,
                    next: alloc.s2048,
                    mark: PhantomData,
                };
                alloc.s2048 = node;
            }
        }
    }
}

impl Default for StringAllocator {
    fn default() -> Self {
        Self::new()
    }
}
