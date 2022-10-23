pub mod nohasher;
pub mod string_interner;
pub mod utf16;
pub mod pointer_vec;

/// offset_of!(type, field); return memory offset of field in usize
#[macro_export]
macro_rules! offset_of {
    ($t:ty, $field:ident) => {{
        const a: $t = unsafe { std::mem::zeroed() };
        &a.$field as *const _ as usize - &a as *const _ as usize
    }};
}

#[repr(C)]
pub struct List<T> {
    pointer: *mut T,
    len: usize,
}

impl<T> AsRef<[T]> for List<T> {
    fn as_ref(&self) -> &[T] {
        unsafe { std::slice::from_raw_parts(self.pointer, self.len) }
    }
}

impl<T> std::ops::Deref for List<T> {
    type Target = [T];
    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

pub struct CombindIterator<I> {
    iters: Vec<I>,
    index: usize,
}

impl<I> CombindIterator<I> {
    pub fn new(iters: Vec<I>) -> Self {
        Self { iters, index: 0 }
    }
}

impl<I> Iterator for CombindIterator<I>
where
    I: Iterator,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(i) = self.iters.get_mut(self.index) {
            match i.next() {
                Some(v) => Some(v),
                None => {
                    self.index += 1;
                    self.next()
                }
            }
        } else {
            None
        }
    }
}

#[repr(transparent)]
pub struct ReferenceRange<T>(std::ops::Range<T>);

impl<T> AsRef<ReferenceRange<T>> for std::ops::Range<T> {
    fn as_ref(&self) -> &ReferenceRange<T> {
        unsafe { std::mem::transmute_copy(&self) }
    }
}

impl<T> IntoIterator for ReferenceRange<T>
where
    std::ops::Range<T>: IntoIterator,
{
    type Item = <std::ops::Range<T> as IntoIterator>::Item;
    type IntoIter = <std::ops::Range<T> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a, T> IntoIterator for &'a ReferenceRange<T>
where
    std::ops::Range<T>: IntoIterator,
    T: Clone,
{
    type Item = <std::ops::Range<T> as IntoIterator>::Item;
    type IntoIter = <std::ops::Range<T> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.clone().into_iter()
    }
}
