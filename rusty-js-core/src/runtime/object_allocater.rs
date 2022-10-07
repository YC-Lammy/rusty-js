use std::alloc::Layout;

use crate::bultins::object::JObjectInner;

use super::gc::GcFlag;

const OBJ_SIZE: usize = std::mem::size_of::<JObjectInner>();
const PAGE_SIZE: usize = 4096;
const OBJ_PER_PAGE: usize = (4096 / OBJ_SIZE) - 1;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct LinkNode {
    flag: GcFlag,
    next: *mut LinkNode,
}

pub struct ObjectAllocator {
    pages: Vec<memmap2::MmapMut>,
    next: *mut LinkNode,
}

impl ObjectAllocator {
    pub unsafe fn allocate(&mut self) -> &'static mut JObjectInner {
        if self.next.is_null() {
            self.add_page();
        };
        let ptr = self.next;
        self.next = (*ptr).next;

        let ptr = ptr as *mut JObjectInner;
        ptr.write(JObjectInner::default());

        return ptr.as_mut().unwrap();
    }

    unsafe fn add_page(&mut self) {
        let map = memmap2::MmapMut::map_anon(PAGE_SIZE).unwrap();
        self.pages.push(map);

        let p = self.pages.last_mut().unwrap().as_mut_ptr() as *mut [JObjectInner; OBJ_PER_PAGE];

        for i in 0..OBJ_PER_PAGE {
            let obj = (p as *mut JObjectInner).add(i);
            let node = obj as *mut JObjectInner as *mut LinkNode;
            *node = LinkNode {
                flag: GcFlag::NotUsed,
                next: self.next,
            };
            self.next = node;
        }
    }

    /// * Mark all the used objects to old
    /// * Mark all the old object to NotUsed
    pub fn marking(&mut self) {
        for i in &mut self.pages {
            let p = unsafe {
                std::slice::from_raw_parts_mut(i.as_mut_ptr() as *mut JObjectInner, OBJ_PER_PAGE)
            };
            for obj in p.iter_mut() {
                if obj.flag == GcFlag::Used {
                    obj.flag = GcFlag::Old;
                } else if obj.flag == GcFlag::Old {
                    obj.flag = GcFlag::NotUsed;
                }
            }
        }
    }

    pub fn garbage_collect(&mut self) {
        for i in &mut self.pages {
            let p = unsafe {
                std::slice::from_raw_parts_mut(i.as_mut_ptr() as *mut JObjectInner, OBJ_PER_PAGE)
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
                }
            }
        }
    }
}

/*
impl Drop for ObjectAllocator{
    fn drop(&mut self) {
        for i in &mut self.pages{
            for i in i.iter_mut(){
                if i.flag != GcFlag::NotUsed{
                    // read and drop
                    let obj = unsafe{std::ptr::read(i)};
                    drop(obj);
                }
            }
            unsafe{Box::from_raw(*i as *mut _ as *mut [[u8;OBJ_SIZE];128])};
        }
    }
}*/

impl Default for ObjectAllocator {
    fn default() -> Self {
        Self {
            pages: Vec::new(),
            next: std::ptr::null_mut(),
        }
    }
}
