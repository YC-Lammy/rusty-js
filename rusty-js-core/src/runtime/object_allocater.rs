use std::{alloc::Layout, sync::Arc};

use crate::{bultins::object::JObjectInner, Runtime};

use super::gc::GcFlag;

const OBJ_SIZE: usize = std::mem::size_of::<JObjectInner>();
/// a page is 4096 bytes
const PAGE_SIZE: usize = 4096;
const OBJ_PER_PAGE: usize = (PAGE_SIZE as f64 / OBJ_SIZE as f64) as usize;

pub struct ObjectAllocator {
    pages: Vec<Box<[JObjectInner; 128]>>,
    next: Option<&'static JObjectInner>,
    alloc_count: u16
}

impl ObjectAllocator {
    pub unsafe fn allocate(&mut self, runtime:Arc<Runtime>) -> &'static mut JObjectInner {
        if self.next.is_none() {
            self.add_page(self.pages.len());
        };
        self.alloc_count += 1;

        if self.alloc_count == 5000{
            self.alloc_count = 0;
            runtime.run_gc();
        };
        let ptr = &mut *(self.next.unwrap() as *const JObjectInner as *mut JObjectInner);

        self.next = std::mem::transmute(ptr.__proto__);
        ptr.__proto__ = None;
        ptr.flag = GcFlag::Used;

        return &mut *ptr;
    }

    unsafe fn add_page(&mut self, num: usize) {
        let mut map = std::alloc::alloc(Layout::array::<JObjectInner>(128 * num).unwrap())
            as *mut JObjectInner;
        self.pages.reserve(num);

        for _ in 0..num {
            let ptr = map as *mut [JObjectInner; 128];
            self.pages.push(Box::from_raw(ptr));
            let slice = &mut *ptr;

            for obj in slice {
                (obj as *mut JObjectInner).write(JObjectInner::default());
                obj.__proto__ = std::mem::transmute(self.next);
                self.next = Some(obj);
            }

            map = map.add(128);
        }

        //if self.pages.len()
    }

    pub fn garbage_collect(&mut self) {
        for i in &mut self.pages {
            let p = unsafe {
                std::slice::from_raw_parts_mut(i.as_ptr() as *mut JObjectInner, OBJ_PER_PAGE)
            };

            for obj in p.iter_mut() {
                if obj.flag == GcFlag::NotUsed {
                    obj.flag = GcFlag::Garbage;
                    obj.__proto__ = unsafe { std::mem::transmute(self.next) };
                    obj.extensible = true;
                    obj.values.clear();
                    obj.wrapped_value = Default::default();

                    self.next = Some(obj);
                } else if obj.flag == GcFlag::Old {
                    obj.flag = GcFlag::NotUsed;
                } else if obj.flag == GcFlag::Used {
                    obj.flag = GcFlag::Old;
                }
            }
        }
    }
}

impl Default for ObjectAllocator {
    fn default() -> Self {
        let mut s = Self {
            pages: Vec::new(),
            next: None,
            alloc_count:0,
        };
        unsafe { s.add_page(1) };
        s
    }
}