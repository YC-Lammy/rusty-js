
// | capacity | length | T...
/// a Vec that has a size of pointer type
pub struct PointerVec<T>(*mut T);


impl<T> PointerVec<T>{
    pub const fn new() -> Self{
        Self(0 as _)
    }

    pub fn with_capacity(capacity: usize) -> Self{
        unsafe{
            let ptr = std::alloc::alloc(std::alloc::Layout::array::<u8>(std::mem::size_of::<[usize;2]>() + std::mem::size_of::<T>()*capacity).unwrap()) as *mut usize;
            *ptr = capacity;
            *ptr.add(1) = 0;
            Self(ptr.add(2) as *mut T)
        }
    }

    pub fn len(&self) -> usize{
        if self.0.is_null(){
            return 0
        }

        unsafe{*(self.0 as *mut usize).sub(1)}
    }

    pub fn capacity(&self) -> usize{
        if self.0.is_null(){
            return 0
        }

        unsafe{*(self.0 as *mut usize).sub(2)}
    }

    pub fn reserve(&mut self, additional: usize) {
        if usize::MAX - additional > self.len(){
            panic!("new capacity cannot exceed usize::Max")
        }
        while (self.capacity() - self.len()) < additional{
            self.realloc();
        }
    }

    pub fn reserve_exact(&mut self, additional: usize){
        if usize::MAX - additional > self.len(){
            panic!("new capacity cannot exceed usize::Max")
        }
        self.reserve(additional)
    }

    pub fn try_reserve(&mut self, additional: usize) -> Result<(), ()>{
        if usize::MAX - additional > self.len(){
            return Err(())
        }
        self.reserve(additional);
        return Ok(())
    }

    pub fn try_reserve_exact(&mut self, additional: usize) -> Result<(), ()>{
        if usize::MAX - additional > self.len(){
            return Err(())
        }
        self.reserve_exact(additional);
        return Ok(())
    }

    pub fn shrink_to_fit(&mut self){

    }

    pub fn shrink_to(&mut self, min_capacity: usize) {
        if self.capacity() > min_capacity {
            let new_cap = self.len().max(min_capacity);
        }
    }

    pub fn into_boxed_slice(mut self) -> Box<[T]> {
        unsafe {
            self.shrink_to_fit();
            let ptr = std::slice::from_raw_parts_mut(self.0, self.len());
            core::mem::forget(self);
            Box::from_raw(ptr)
        }
    }

    pub fn truncate(&mut self, len: usize) {
        // This is safe because:
        //
        // * the slice passed to `drop_in_place` is valid; the `len > self.len`
        //   case avoids creating an invalid slice, and
        // * the `len` of the vector is shrunk before calling `drop_in_place`,
        //   such that no value will be dropped twice in case `drop_in_place`
        //   were to panic once (if it panics twice, the program aborts).
        unsafe {
            // Note: It's intentional that this is `>` and not `>=`.
            //       Changing it to `>=` has negative performance
            //       implications in some cases. See #78884 for more.
            if len > self.len() {
                return;
            }
            let remaining_len = self.len() - len;
            let s = std::slice::from_raw_parts_mut(self.0.add(len), remaining_len);
            *((self.0 as *mut usize).sub(1)) = len;
            core::ptr::drop_in_place(s as *mut [T]);
        }
    }

    pub fn as_slice(&self) -> &[T] {
        self
    }

    pub fn as_mut_slice(&mut self) -> &mut [T]{
        self
    }

    pub fn as_ptr(&self) -> *const T {
        // We shadow the slice method of the same name to avoid going through
        // `deref`, which creates an intermediate reference.
        if self.0.is_null(){
            return core::ptr::NonNull::dangling().as_ptr()
        }
        self.0
    }

    pub fn as_mut_ptr(&self) -> *mut T {
        // We shadow the slice method of the same name to avoid going through
        // `deref`, which creates an intermediate reference.
        if self.0.is_null(){
            return core::ptr::NonNull::dangling().as_ptr()
        }
        self.0
    }

    pub unsafe fn set_len(&mut self, new_len: usize) {
        debug_assert!(new_len <= self.capacity());

        *((self.0 as *mut usize).sub(1)) = new_len;
    }

    pub fn swap_remove(&mut self, index: usize) -> T {
        #[cold]
        #[inline(never)]
        fn assert_failed(index: usize, len: usize) -> ! {
            panic!("swap_remove index (is {index}) should be < len (is {len})");
        }

        let len = self.len();
        if index >= len {
            assert_failed(index, len);
        }
        unsafe {
            // We replace self[index] with the last element. Note that if the
            // bounds check above succeeds there must be a last element (which
            // can be self[index] itself).
            let value = core::ptr::read(self.as_ptr().add(index));
            let base_ptr = self.as_mut_ptr();
            core::ptr::copy(base_ptr.add(len - 1), base_ptr.add(index), 1);
            self.set_len(len - 1);
            value
        }
    }

    pub fn insert(&mut self, index: usize, element: T) {
        #[cold]
        #[inline(never)]
        fn assert_failed(index: usize, len: usize) -> ! {
            panic!("insertion index (is {index}) should be <= len (is {len})");
        }

        let len = self.len();
        if index > len {
            assert_failed(index, len);
        }

        // space for the new element
        if len == self.capacity() {
            self.reserve(1);
        }

        unsafe {
            // infallible
            // The spot to put the new value
            {
                let p = self.as_mut_ptr().add(index);
                // Shift everything over to make space. (Duplicating the
                // `index`th element into two consecutive places.)
                core::ptr::copy(p, p.offset(1), len - index);
                // Write it in, overwriting the first copy of the `index`th
                // element.
                core::ptr::write(p, element);
            }
            self.set_len(len + 1);
        }
    }

    pub fn remove(&mut self, index: usize) -> T {
        #[cold]
        #[inline(never)]
        #[track_caller]
        fn assert_failed(index: usize, len: usize) -> ! {
            panic!("removal index (is {index}) should be < len (is {len})");
        }

        let len = self.len();
        if index >= len {
            assert_failed(index, len);
        }
        unsafe {
            // infallible
            let ret;
            {
                // the place we are taking from.
                let ptr = self.as_mut_ptr().add(index);
                // copy it out, unsafely having a copy of the value on
                // the stack and in the vector at the same time.
                ret = core::ptr::read(ptr);

                // Shift everything down to fill in that spot.
                core::ptr::copy(ptr.offset(1), ptr, len - index - 1);
            }
            self.set_len(len - 1);
            ret
        }
    }

    pub fn retain<F>(&mut self, mut f: F)
    where
        F: FnMut(&T) -> bool,
    {
        self.retain_mut(|elem| f(elem));
    }

    pub fn retain_mut<F>(&mut self, mut f: F)
    where
        F: FnMut(&mut T) -> bool,
    {
        let original_len = self.len();
        // Avoid double drop if the drop guard is not executed,
        // since we may make some holes during the process.
        unsafe { self.set_len(0) };

        // Vec: [Kept, Kept, Hole, Hole, Hole, Hole, Unchecked, Unchecked]
        //      |<-              processed len   ->| ^- next to check
        //                  |<-  deleted cnt     ->|
        //      |<-              original_len                          ->|
        // Kept: Elements which predicate returns true on.
        // Hole: Moved or dropped element slot.
        // Unchecked: Unchecked valid elements.
        //
        // This drop guard will be invoked when predicate or `drop` of element panicked.
        // It shifts unchecked elements to cover holes and `set_len` to the correct length.
        // In cases when predicate and `drop` never panick, it will be optimized out.
        struct BackshiftOnDrop<'a, T> {
            v: &'a mut PointerVec<T>,
            processed_len: usize,
            deleted_cnt: usize,
            original_len: usize,
        }

        impl<T> Drop for BackshiftOnDrop<'_, T> {
            fn drop(&mut self) {
                if self.deleted_cnt > 0 {
                    // SAFETY: Trailing unchecked items must be valid since we never touch them.
                    unsafe {
                        core::ptr::copy(
                            self.v.as_ptr().add(self.processed_len),
                            self.v.as_mut_ptr().add(self.processed_len - self.deleted_cnt),
                            self.original_len - self.processed_len,
                        );
                    }
                }
                // SAFETY: After filling holes, all items are in contiguous memory.
                unsafe {
                    self.v.set_len(self.original_len - self.deleted_cnt);
                }
            }
        }

        let mut g = BackshiftOnDrop { v: self, processed_len: 0, deleted_cnt: 0, original_len };

        fn process_loop<F, T, const DELETED: bool>(
            original_len: usize,
            f: &mut F,
            g: &mut BackshiftOnDrop<'_, T>,
        ) where
            F: FnMut(&mut T) -> bool,
        {
            while g.processed_len != original_len {
                // SAFETY: Unchecked element must be valid.
                let cur = unsafe { &mut *g.v.as_mut_ptr().add(g.processed_len) };
                if !f(cur) {
                    // Advance early to avoid double drop if `drop_in_place` panicked.
                    g.processed_len += 1;
                    g.deleted_cnt += 1;
                    // SAFETY: We never touch this element again after dropped.
                    unsafe { core::ptr::drop_in_place(cur) };
                    // We already advanced the counter.
                    if DELETED {
                        continue;
                    } else {
                        break;
                    }
                }
                if DELETED {
                    // SAFETY: `deleted_cnt` > 0, so the hole slot must not overlap with current element.
                    // We use copy for move, and never touch this element again.
                    unsafe {
                        let hole_slot = g.v.as_mut_ptr().add(g.processed_len - g.deleted_cnt);
                        core::ptr::copy_nonoverlapping(cur, hole_slot, 1);
                    }
                }
                g.processed_len += 1;
            }
        }

        // Stage 1: Nothing was deleted.
        process_loop::<F, T, false>(original_len, &mut f, &mut g);

        // Stage 2: Some elements were deleted.
        process_loop::<F, T, true>(original_len, &mut f, &mut g);

        // All item are processed. This can be optimized to `set_len` by LLVM.
        drop(g);
    }

    pub fn dedup_by_key<F, K>(&mut self, mut key: F)
    where
        F: FnMut(&mut T) -> K,
        K: PartialEq,
    {
        self.dedup_by(|a, b| key(a) == key(b))
    }

    pub fn dedup_by<F>(&mut self, mut same_bucket: F)
    where
        F: FnMut(&mut T, &mut T) -> bool,
    {
        let len = self.len();
        if len <= 1 {
            return;
        }

        /* INVARIANT: vec.len() > read >= write > write-1 >= 0 */
        struct FillGapOnDrop<'a, T> {
            /* Offset of the element we want to check if it is duplicate */
            read: usize,

            /* Offset of the place where we want to place the non-duplicate
             * when we find it. */
            write: usize,

            /* The Vec that would need correction if `same_bucket` panicked */
            vec: &'a mut PointerVec<T>,
        }

        impl<'a, T> Drop for FillGapOnDrop<'a, T> {
            fn drop(&mut self) {
                /* This code gets executed when `same_bucket` panics */

                /* SAFETY: invariant guarantees that `read - write`
                 * and `len - read` never overflow and that the copy is always
                 * in-bounds. */
                unsafe {
                    let ptr = self.vec.as_mut_ptr();
                    let len = self.vec.len();

                    /* How many items were left when `same_bucket` panicked.
                     * Basically vec[read..].len() */
                    let items_left = len.wrapping_sub(self.read);

                    /* Pointer to first item in vec[write..write+items_left] slice */
                    let dropped_ptr = ptr.add(self.write);
                    /* Pointer to first item in vec[read..] slice */
                    let valid_ptr = ptr.add(self.read);

                    /* Copy `vec[read..]` to `vec[write..write+items_left]`.
                     * The slices can overlap, so `copy_nonoverlapping` cannot be used */
                    core::ptr::copy(valid_ptr, dropped_ptr, items_left);

                    /* How many items have been already dropped
                     * Basically vec[read..write].len() */
                    let dropped = self.read.wrapping_sub(self.write);

                    self.vec.set_len(len - dropped);
                }
            }
        }

        let mut gap = FillGapOnDrop { read: 1, write: 1, vec: self };
        let ptr = gap.vec.as_mut_ptr();

        /* Drop items while going through Vec, it should be more efficient than
         * doing slice partition_dedup + truncate */

        /* SAFETY: Because of the invariant, read_ptr, prev_ptr and write_ptr
         * are always in-bounds and read_ptr never aliases prev_ptr */
        unsafe {
            while gap.read < len {
                let read_ptr = ptr.add(gap.read);
                let prev_ptr = ptr.add(gap.write.wrapping_sub(1));

                if same_bucket(&mut *read_ptr, &mut *prev_ptr) {
                    // Increase `gap.read` now since the drop may panic.
                    gap.read += 1;
                    /* We have found duplicate, drop it in-place */
                    core::ptr::drop_in_place(read_ptr);
                } else {
                    let write_ptr = ptr.add(gap.write);

                    /* Because `read_ptr` can be equal to `write_ptr`, we either
                     * have to use `copy` or conditional `copy_nonoverlapping`.
                     * Looks like the first option is faster. */
                    core::ptr::copy(read_ptr, write_ptr, 1);

                    /* We have filled that place, so go further */
                    gap.write += 1;
                    gap.read += 1;
                }
            }

            /* Technically we could let `gap` clean up with its Drop, but
             * when `same_bucket` is guaranteed to not panic, this bloats a little
             * the codegen, so we just do it manually */
            gap.vec.set_len(gap.write);
            core::mem::forget(gap);
        }
    }


    fn realloc(&mut self){
        unsafe{
            if self.0.is_null(){
                let ptr = std::alloc::alloc(std::alloc::Layout::array::<u8>(std::mem::size_of::<[usize;2]>() + std::mem::size_of::<T>()*24).unwrap()) as *mut usize;
                *ptr = 24;
                *ptr.add(1) = 0;
                self.0 = ptr.add(2) as *mut T;
            } else{
                let l = self.len();
                let cap = self.capacity();
                let ptr = std::alloc::alloc(std::alloc::Layout::array::<u8>(
                    std::mem::size_of::<[usize;2]>() + std::mem::size_of::<T>()*cap*2).unwrap()) as *mut usize;
                *ptr = cap*2;
                *ptr.add(1) = l;
                self.0 = ptr.add(2) as *mut T;
            }
        }
        
    }

    pub fn push(&mut self, value:T){
        let l = self.len();
        if l == self.capacity(){
            self.realloc();
        }
        unsafe{*(self.0.add(l)) = value};
    }

    pub fn pop(&mut self) -> Option<T> {
        let l = self.len();
        if l == 0 {
            None
        } else {
            unsafe {
                self.set_len(l-1);
                Some(core::ptr::read(self.as_ptr().add(self.len())))
            }
        }
    }

    pub fn append(&mut self, other: &mut Self) {
        unsafe {
            self.append_elements(other.as_slice() as _);
            other.set_len(0);
        }
    }

    unsafe fn append_elements(&mut self, other: *const [T]) {
        let count = (*other).len() ;
        self.reserve(count);
        let len = self.len();
        core::ptr::copy_nonoverlapping(other as *const T, self.as_mut_ptr().add(len), count) ;
        self.set_len(len + count);
    }

    pub fn drain<R>(&mut self, range: R) -> std::vec::Drain<'_, T>
    where
        R: core::ops::RangeBounds<usize>,
    {
        // Memory safety
        //
        // When the Drain is first created, it shortens the length of
        // the source vector to make sure no uninitialized or moved-from elements
        // are accessible at all if the Drain's destructor never gets to run.
        //
        // Drain will ptr::read out the values to remove.
        // When finished, remaining tail of the vec is copied back to cover
        // the hole, and the vector length is restored to the new length.
        //
        let len = self.len();
        let bounds = ..len;
        let (start, end ) = {
            let len = bounds.end;

            let start: std::ops::Bound<&usize> = range.start_bound();
            let start = match start {
                std::ops::Bound::Included(&start) => start,
                std::ops::Bound::Excluded(start) => {
                    start.checked_add(1).unwrap_or_else(|| panic!("attempted to index slice from after maximum usize"))
                }
                std::ops::Bound::Unbounded => 0,
            };

            let end: std::ops::Bound<&usize> = range.end_bound();
            let end = match end {
                std::ops::Bound::Included(end) => {
                    end.checked_add(1).unwrap_or_else(|| panic!("attempted to index slice up to maximum usize"))
                },
                std::ops::Bound::Excluded(&end) => end,
                std::ops::Bound::Unbounded => len,
            };

            if start > end {
                panic!("slice index start is larger than end");
            }
            if end > len {
                panic!("slice start index is out of range for slice");
            }

            (start, end)
        };

        unsafe {
            // set self.vec length's to start, to be safe in case Drain is leaked
            self.set_len(start);
            // Use the borrow in the IterMut to indicate borrowing behavior of the
            // whole Drain iterator (like &mut T).
            let range_slice = std::slice::from_raw_parts_mut(self.as_mut_ptr().add(start), end - start);
            todo!()
        }
    }

    pub fn clear(&mut self) {
        let elems: *mut [T] = self.as_mut_slice();

        // SAFETY:
        // - `elems` comes directly from `as_mut_slice` and is therefore valid.
        // - Setting `self.len` before calling `drop_in_place` means that,
        //   if an element's `Drop` impl panics, the vector's `Drop` impl will
        //   do nothing (leaking the rest of the elements) instead of dropping
        //   some twice.
        unsafe {
            self.set_len(0);
            core::ptr::drop_in_place(elems);
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn split_off(&mut self, at: usize) -> Self
    {
        #[cold]
        #[inline(never)]
        fn assert_failed(at: usize, len: usize) -> ! {
            panic!("`at` split index (is {at}) should be <= len (is {len})");
        }

        if at > self.len() {
            assert_failed(at, self.len());
        }

        if at == 0 {
            // the new vector can take over the original buffer and avoid the copy
            return std::mem::replace(
                self,
                Self::with_capacity(self.capacity()),
            );
        }

        let other_len = self.len() - at;
        let mut other = Self::with_capacity(other_len);

        // Unsafely `set_len` and copy items to `other`.
        unsafe {
            self.set_len(at);
            other.set_len(other_len);

            std::ptr::copy_nonoverlapping(self.as_ptr().add(at), other.as_mut_ptr(), other.len());
        }
        other
    }

    pub fn resize_with<F>(&mut self, new_len: usize, mut f: F)
    where
        F: FnMut() -> T,
    {
        let len = self.len();
        if new_len > len {
            let l = new_len - len;
            self.reserve(l);
            for _ in 0..l{
                self.push((f)());
            }
        } else {
            self.truncate(new_len);
        }
    }

    pub fn leak<'a>(self) -> &'a mut [T]
    {
        let me = std::mem::ManuallyDrop::new(self);
        unsafe { std::slice::from_raw_parts_mut(me.as_mut_ptr(), me.len()) }
    }

    pub fn spare_capacity_mut(&mut self) -> &mut [std::mem::MaybeUninit<T>] {
        // Note:
        // This method is not implemented in terms of `split_at_spare_mut`,
        // to prevent invalidation of pointers to the buffer.
        unsafe {
            std::slice::from_raw_parts_mut(
                self.as_mut_ptr().add(self.len()) as *mut std::mem::MaybeUninit<T>,
                self.capacity() - self.len(),
            )
        }
    }

    pub fn split_at_spare_mut(&mut self) -> (&mut [T], &mut [std::mem::MaybeUninit<T>]) {
        // SAFETY:
        // - len is ignored and so never changed
        let (init, spare, _) = unsafe { self.split_at_spare_mut_with_len() };
        (init, spare)
    }

    unsafe fn split_at_spare_mut_with_len(
        &mut self,
    ) -> (&mut [T], &mut [std::mem::MaybeUninit<T>], &mut usize) {
        let ptr = self.as_mut_ptr();
        // SAFETY:
        // - `ptr` is guaranteed to be valid for `self.len` elements
        // - but the allocation extends out to `self.buf.capacity()` elements, possibly
        // uninitialized
        let spare_ptr = ptr.add(self.len()) ;
        let spare_ptr = spare_ptr.cast::<std::mem::MaybeUninit<T>>();
        let spare_len = self.capacity() - self.len();

        // SAFETY:
        // - `ptr` is guaranteed to be valid for `self.len` elements
        // - `spare_ptr` is pointing one element past the buffer, so it doesn't overlap with `initialized`
        let initialized = std::slice::from_raw_parts_mut(ptr, self.len());
        let spare = std::slice::from_raw_parts_mut(spare_ptr, spare_len);

        (initialized, spare, (self.0 as *mut usize).sub(1).as_mut().unwrap())
        
    }

    
}

impl<T:Clone> PointerVec<T>{
    pub fn resize(&mut self, new_len: usize, value: T){
        let len = self.len();

        if new_len > len {
            let l = new_len - len;
            self.reserve(l);
            unsafe{
                let p = self.as_mut_ptr().add(len);
                for i in 0..l{
                    p.add(i).write(value.clone())
                }
                self.set_len(new_len);
            }
            
        } else {
            self.truncate(new_len);
        }
    }

    pub fn extend_from_slice(&mut self, other: &[T]){
        self.reserve(other.len());
        unsafe{
            let l = self.len();
            let mut i = 0;
            for v in other{
                self.0.add(l).add(i).write(v.clone());
                i += 1;
            }
            self.set_len(l+other.len())
        }
        
    }

    pub fn extend_from_within<R>(&mut self, src: R)
    where
        R: std::ops::RangeBounds<usize>
    {
        let start = match src.start_bound(){
            std::ops::Bound::Excluded(v) => *v,
            std::ops::Bound::Included(v) => *v,
            std::ops::Bound::Unbounded => 0
        };
        let end = match src.end_bound(){
            std::ops::Bound::Excluded(v) => *v,
            std::ops::Bound::Included(v) => *v +1,
            std::ops::Bound::Unbounded => self.len()
        };
        let s = unsafe{std::slice::from_raw_parts(self.0, self.len())};
        self.extend_from_slice(&s[start..end]);
    }
}

impl<T> Drop for PointerVec<T>{
    fn drop(&mut self) {
        unsafe{
            let s = self.as_mut_slice() as *mut [T];
            std::ptr::drop_in_place(s);
            std::alloc::dealloc(self.0 as *mut u8, 
                std::alloc::Layout::array::<u8>(std::mem::size_of::<[usize;2]>() + std::mem::size_of::<T>()*self.capacity()).unwrap());
        }
    }
}

impl<T> std::ops::Deref for PointerVec<T> {
    type Target = [T];

    fn deref(&self) -> &[T] {
        unsafe { std::slice::from_raw_parts(self.as_ptr(), self.len()) }
    }
}

impl<T> std::ops::DerefMut for PointerVec<T> {

    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { std::slice::from_raw_parts_mut(self.as_mut_ptr(), self.len()) }
    }
}

impl<T: Clone> Clone for PointerVec<T> {
    fn clone(&self) -> Self {
        let mut a = Self::new();
        a.reserve(self.capacity());
        unsafe{
            for i in 0..self.len(){
                a.0.add(i).write(self[i].clone());
            };
            a.set_len(self.len());
        };
        return a
    }
}

impl<T: std::hash::Hash> std::hash::Hash for PointerVec<T> {
    #[inline]
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::hash::Hash::hash(&**self, state)
    }
}

impl<T, I: std::slice::SliceIndex<[T]>> std::ops::Index<I> for PointerVec<T> {
    type Output = I::Output;

    #[inline]
    fn index(&self, index: I) -> &Self::Output {
        std::ops::Index::index(&**self, index)
    }
}

impl<T, I: std::slice::SliceIndex<[T]>> std::ops::IndexMut<I> for PointerVec<T> {
    #[inline]
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        std::ops::IndexMut::index_mut(&mut **self, index)
    }
}

impl<A> std::iter::FromIterator<A> for PointerVec<A>{
    fn from_iter<T: IntoIterator<Item = A>>(iter: T) -> Self {
        let mut a = Self::new();
        iter.into_iter().for_each(|v|{
            a.push(v);
        });
        return a;
    }
}


impl<'a, T> IntoIterator for &'a PointerVec<T>{
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        self.as_slice().into_iter()
    }
}

impl<'a, T> IntoIterator for &'a mut PointerVec<T>{
    type Item = &'a mut T;
    type IntoIter = std::slice::IterMut<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        self.as_mut_slice().into_iter()
    }
}

impl<T> IntoIterator for PointerVec<T>{
    type IntoIter = Iter<T>;
    type Item = T;

    fn into_iter(self) -> Self::IntoIter {
        let i = Iter{
            v:self.0,
            len:self.len(),
            count:0
        };
        // prevent drop of values
        std::mem::forget(self);
        i
    }
}

pub struct Iter<T>{
    v:*mut T,
    len:usize,
    count:usize
}

impl<T> Iterator for Iter<T>{
    type Item = T;

    fn count(self) -> usize
        where
            Self: Sized, {
        return self.len - self.count
    }


    fn next(&mut self) -> Option<Self::Item> {
        if self.count >= self.len{
            return None
        }
        unsafe{
            let v = self.v.add(self.count).read();
            self.count += 1;
            return Some(v)
        }
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        if n >= self.len - self.count{
            return None
        }

        return Some(unsafe{self.v.add(self.count + n).read()})
    }
}

impl<T> Drop for Iter<T>{
    fn drop(&mut self) {
        let mut v = PointerVec(self.v);
        unsafe{v.set_len(0)};
        drop(v);
    }
}