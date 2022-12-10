use std::alloc::Layout;

use crate::bultins::JSBigInt;

use super::GcFlag;

#[repr(C)]
pub struct Node {
    flag: GcFlag,
    next: *mut Node,
}

pub struct BigIntAllocator {
    pages: Vec<Box<[JSBigInt; 128]>>,
    next: *mut Node,
}

impl Default for BigIntAllocator {
    fn default() -> Self {
        Self {
            pages: Default::default(),
            next: std::ptr::null_mut(),
        }
    }
}

impl BigIntAllocator {
    pub fn alloc(&mut self) -> &'static mut JSBigInt {
        unsafe {
            if self.next.is_null() {
                self.add_pages(self.pages.len());
            }

            let next = self.next;
            self.next = (*next).next;

            let b = next as *mut JSBigInt;
            b.write(JSBigInt::zero());

            &mut *b
        }
    }

    unsafe fn add_pages(&mut self, num: usize) {
        let mut ptr = std::alloc::alloc(Layout::array::<JSBigInt>(128 * num).unwrap())
            as *mut [JSBigInt; 128];

        for _ in 0..num {
            let slice = &mut *ptr;
            self.pages.push(Box::from_raw(ptr));

            for i in slice {
                let n = i as *mut _ as *mut Node;
                n.write(Node {
                    flag: GcFlag::Garbage,
                    next: self.next,
                });
                self.next = n;
            }
            ptr = ptr.add(1);
        }
    }

    pub fn garbage_collect(&mut self) {
        for page in &mut self.pages {
            for b in page.iter_mut() {
                if b.flag == GcFlag::NotUsed {
                    b.flag = GcFlag::Garbage;
                    let next = self.next;
                    self.next = b as *mut _ as *mut Node;
                    unsafe {
                        (&mut *self.next).next = next;
                    }
                }
            }
        }
    }
}
