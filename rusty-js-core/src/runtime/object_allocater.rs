use std::alloc::Layout;

use crate::bultins::object::JObjectInner;

use super::gc::GcFlag;

const OBJ_SIZE: usize = std::mem::size_of::<JObjectInner>();
/// a page is 4096 bytes
const PAGE_SIZE: usize = 4096;
const OBJ_PER_PAGE: usize = (4096 as f64/ OBJ_SIZE as f64) as usize;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct LinkNode {
    flag: GcFlag,
    next: *mut LinkNode,
}

pub struct ObjectAllocator {
    pages: Vec<&'static [u8;4096]>,
    next: *mut LinkNode,
}

impl ObjectAllocator {
    pub unsafe fn allocate(&mut self) -> &'static mut JObjectInner {
        if self.next.is_null() {
            self.add_page(self.pages.len());
        };
        let ptr = self.next;
        self.next = (*ptr).next;

        let ptr = ptr as *mut JObjectInner;
        ptr.write(JObjectInner::default());

        return ptr.as_mut().unwrap();
    }

    unsafe fn add_page(&mut self, num:usize) {
        let map = std::alloc::alloc(Layout::new::<[u8;4096]>());
        self.pages.push((map as *mut [u8;4096]).as_mut().unwrap());

        let mut p = self.pages.last_mut().unwrap().as_ptr() as *mut [JObjectInner; OBJ_PER_PAGE];

        for _ in 0..num{
            for i in 0..OBJ_PER_PAGE {
                let obj = (p as *mut JObjectInner).add(i);
                let node = obj as *mut JObjectInner as *mut LinkNode;
                *node = LinkNode {
                    flag: GcFlag::NotUsed,
                    next: self.next,
                };
                self.next = node;
            }
            p = (p as *mut u8).add(4096) as *mut _;
        }
        
    }

    pub fn garbage_collect(&mut self) {
        for i in &mut self.pages {
            let p = unsafe {
                std::slice::from_raw_parts_mut(i.as_ptr() as *mut JObjectInner, OBJ_PER_PAGE)
            };

            for obj in p.iter_mut() {
                if obj.flag == GcFlag::NotUsed {
                    let o = unsafe { std::ptr::read(obj) };
                    drop(o);

                    let node = obj as *mut JObjectInner as *mut LinkNode;
                    unsafe {
                        *node = LinkNode {
                            flag: GcFlag::Garbage,
                            next: self.next,
                        }
                    };
                    self.next = node;

                } else if obj.flag == GcFlag::Old{
                    obj.flag = GcFlag::NotUsed;
                } else if obj.flag == GcFlag::Used{
                    obj.flag = GcFlag::Old;
                }
            }
        }
    }
}


impl Drop for ObjectAllocator{
    fn drop(&mut self) {
        for i in &mut self.pages{
            let p = unsafe {
                std::slice::from_raw_parts_mut(i.as_ptr() as *mut JObjectInner, OBJ_PER_PAGE)
            };
            for obj in p.iter_mut(){
                if obj.flag != GcFlag::Garbage{
                    unsafe{drop(std::ptr::read(obj))};
                }
            }
            unsafe{std::alloc::dealloc(i.as_ptr() as *mut u8, Layout::new::<[u8;4096]>())}
        }
    }
}

impl Default for ObjectAllocator {
    fn default() -> Self {
        let mut s = Self {
            pages: Vec::new(),
            next: std::ptr::null_mut(),
        };
        unsafe{s.add_page(1)};
        s
    }
}

#[test]
fn a(){
    println!("{}", OBJ_SIZE);
}