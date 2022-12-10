use std::marker::PhantomData;

#[repr(C)]
pub struct HashTable<K, T> {
    // Mask to get an index from a hash value. The value is one less than the
    // number of buckets in the table.
    bucket_mask: usize,

    // [Padding], T1, T2, ..., Tlast, C1, C2, ...
    //                                ^ points here
    ctrl: *mut u8,

    // Number of elements that can be inserted before we need to grow the table
    growth_left: usize,

    // Number of elements in the table, only really used by len()
    items: usize,

    marker:PhantomData<(K, T)>
}

impl<K, T> HashTable<K, T>{
    fn new(){
        
    }
}