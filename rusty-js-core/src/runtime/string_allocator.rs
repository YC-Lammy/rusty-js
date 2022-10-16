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

    s64: *mut LinkNode<64>,
    s64_pages:Vec<&'static mut [[u8;64]]>,
    s128: *mut LinkNode<128>,
    s128_pages:Vec<&'static mut [[u8;128]]>,
    s256: *mut LinkNode<256>,
    s256_pages:Vec<&'static mut [[u8;256]]>,
    s512: *mut LinkNode<512>,
    s512_pages:Vec<&'static mut [[u8; 512]]>,
    s1024: *mut LinkNode<1024>,
    s1024_pages:Vec<&'static mut [[u8; 1024]]>,
    s2048: *mut LinkNode<2048>,
    s2048_pages:Vec<&'static mut [[u8; 2048]]>,

    sys: Vec<*mut JStringHeader>,
}

impl StringAllocator {
    pub fn new() -> Self {
        Self {
            s64: std::ptr::null_mut(),
            s64_pages:Vec::new(),
            s128: std::ptr::null_mut(),
            s128_pages:Vec::new(),
            s256: std::ptr::null_mut(),
            s256_pages:Vec::new(),
            s512: std::ptr::null_mut(),
            s512_pages:Vec::new(),
            s1024: std::ptr::null_mut(),
            s1024_pages:Vec::new(),
            s2048: std::ptr::null_mut(),
            s2048_pages:Vec::new(),
            sys: Vec::new(),
        }
    }

    #[inline]
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
            // slow allocation
            let v = unsafe { std::alloc::alloc(Layout::array::<u8>(size).unwrap()) };
            self.sys.push(v as _);
            v
        }) as *mut JStringHeader
    }

    pub unsafe fn garbage_collect(&mut self){
        for i in &self.s64_pages{
            for i in i.iter(){
                let n = (i as *const _ as *mut LinkNode<64>).as_mut().unwrap();
                if n.flag == GcFlag::NotUsed{
                    n.flag = GcFlag::Garbage;
                    let next = self.s64;
                    n.next = next;
                    self.s64 = n;

                } else if n.flag == GcFlag::Used{
                    n.flag = GcFlag::Old;
                } else if n.flag == GcFlag::Old{
                    n.flag = GcFlag::NotUsed;
                }
            }
        }

        for i in &self.s128_pages{
            for i in i.iter(){
                let n = (i as *const _ as *mut LinkNode<128>).as_mut().unwrap();
                if n.flag == GcFlag::NotUsed{
                    n.flag = GcFlag::Garbage;
                    let next = self.s128;
                    n.next = next;
                    self.s128 = n;

                } else if n.flag == GcFlag::Used{
                    n.flag = GcFlag::Old;
                } else if n.flag == GcFlag::Old{
                    n.flag = GcFlag::NotUsed;
                }
            }
        }

        for i in &self.s256_pages{
            for i in i.iter(){
                let n = (i as *const _ as *mut LinkNode<256>).as_mut().unwrap();
                if n.flag == GcFlag::NotUsed{
                    n.flag = GcFlag::Garbage;
                    let next = self.s256;
                    n.next = next;
                    self.s256 = n;

                } else if n.flag == GcFlag::Used{
                    n.flag = GcFlag::Old;
                } else if n.flag == GcFlag::Old{
                    n.flag = GcFlag::NotUsed;
                }
            }
        }

        for i in &self.s512_pages{
            for i in i.iter(){
                let n = (i as *const _ as *mut LinkNode<512>).as_mut().unwrap();
                if n.flag == GcFlag::NotUsed{
                    n.flag = GcFlag::Garbage;
                    let next = self.s512;
                    n.next = next;
                    self.s512 = n;

                } else if n.flag == GcFlag::Used{
                    n.flag = GcFlag::Old;
                } else if n.flag == GcFlag::Old{
                    n.flag = GcFlag::NotUsed;
                }
            }
        }

        for i in &self.s1024_pages{
            for i in i.iter(){
                let n = (i as *const _ as *mut LinkNode<1024>).as_mut().unwrap();
                if n.flag == GcFlag::NotUsed{
                    n.flag = GcFlag::Garbage;
                    let next = self.s1024;
                    n.next = next;
                    self.s1024 = n;

                } else if n.flag == GcFlag::Used{
                    n.flag = GcFlag::Old;
                } else if n.flag == GcFlag::Old{
                    n.flag = GcFlag::NotUsed;
                }
            }
        }

        for i in &self.s2048_pages{
            for i in i.iter(){
                let n = (i as *const _ as *mut LinkNode<2048>).as_mut().unwrap();
                if n.flag == GcFlag::NotUsed{
                    n.flag = GcFlag::Garbage;
                    let next = self.s2048;
                    n.next = next;
                    self.s2048 = n;

                } else if n.flag == GcFlag::Used{
                    n.flag = GcFlag::Old;
                } else if n.flag == GcFlag::Old{
                    n.flag = GcFlag::NotUsed;
                }
            }
        }
    }
}

impl<const SIZE: usize> LinkNode<SIZE> {
    #[inline]
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

    #[inline]
    unsafe fn add_page(alloc: &mut StringAllocator) {
        let page = std::alloc::alloc(Layout::new::<[u8; 4096]>()) as *mut [u8; 4096];

        let pages = std::slice::from_raw_parts_mut(page as *mut [u8; SIZE], 4096 / SIZE);
        
        match SIZE{
            64 => alloc.s64_pages.push(std::mem::transmute_copy(&pages)),
            128 => alloc.s128_pages.push(std::mem::transmute_copy(&pages)),
            256 => alloc.s256_pages.push(std::mem::transmute_copy(&pages)),
            512 => alloc.s512_pages.push(std::mem::transmute_copy(&pages)),
            1024 => alloc.s1024_pages.push(std::mem::transmute_copy(&pages)),
            2048 => alloc.s2048_pages.push(std::mem::transmute_copy(&pages)),
            _ => {}
        };

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
