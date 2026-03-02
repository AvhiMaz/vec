/// A heap-allocated growable array built from scratch using raw pointers
/// and manual memory management.
///
/// On the stack, `MyVec<T>` is three words:
///
/// - `ptr` - raw pointer to a contiguous block of heap memory holding `T` values
/// - `len` - number of elements currently written into that block
/// - `cap` - total number of `T` slots the block can hold before reallocation
///
/// The heap block is uninitialized beyond index `len`. Those bytes must never
/// be read until a value is written there with `push`.
pub struct MyVec<T> {
    /// Raw pointer to the start of the heap allocation.
    /// Null when `cap == 0`. Valid for reads and writes within `[0, cap)` after
    /// the first `grow` call.
    pub ptr: *mut T,
    /// Total number of `T` slots available in the current allocation.
    /// Zero means no heap memory has been requested yet.
    pub cap: usize,
    /// Number of elements that have been pushed and not yet popped.
    /// Always less than or equal to `cap`.
    pub len: usize,
}

impl<T> MyVec<T> {
    /// Returns the number of elements currently stored in the vec.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns the total number of elements the vec can hold without reallocating.
    pub fn cap(&self) -> usize {
        self.cap
    }

    /// Creates a new, empty `MyVec<T>` without allocating any heap memory.
    ///
    /// No allocation happens here. The first call to `push` triggers the
    /// initial `grow`.
    pub fn new() -> Self {
        MyVec {
            ptr: std::ptr::null_mut(),
            cap: 0,
            len: 0,
        }
    }

    /// Appends `data` to the end of the vec.
    ///
    /// If the vec is full (`len == cap`), `grow` is called first to double
    /// the capacity. The value is then written directly into heap memory at
    /// offset `len` using `ptr::write`, which copies the bytes without reading
    /// or dropping whatever was previously at that address.
    ///
    /// For ZST, no allocation or write occurs. Only `len` is incremented.
    pub fn push(&mut self, data: T) {
        if core::mem::size_of::<T>() != 0 && self.cap == self.len {
            self.grow();
        }

        if core::mem::size_of::<T>() != 0 {
            unsafe {
                // SAFETY: grow ensures ptr is valid for at least len+1 slots.
                // The slot at len is uninitialized so ptr::write is used to
                // avoid running Drop on whatever bytes are there.
                let dst = self.ptr.add(self.len);
                std::ptr::write(dst, data);
            }
        }
        self.len += 1;
    }

    /// Removes and returns the last element, or `None` if the vec is empty.
    ///
    /// `len` is decremented first so that the slot at the new `len` is the
    /// one being removed. `ptr::read` copies the value out of heap memory
    /// without running `Drop` on the slot, transferring ownership to the caller.
    /// The heap memory for that slot is not freed - only `Drop` (or a future
    /// `push`) will touch it again.
    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            return None;
        } else {
            self.len -= 1;
            unsafe {
                // SAFETY: len was just decremented so self.len points to a valid
                // initialized slot. ptr::read moves ownership out without
                // running Drop on the original slot.
                let value = std::ptr::read(self.ptr.add(self.len));
                return Some(value);
            }
        }
    }

    /// Returns a shared reference to the element at `index`, or `None` if
    /// `index` is out of bounds.
    ///
    /// The bounds check against `len` (not `cap`) ensures we only hand out
    /// references to initialized memory. Slots between `len` and `cap` are
    /// allocated but uninitialized and must never be referenced.
    pub fn get(&self, index: usize) -> Option<&T> {
        if index >= self.len {
            return None;
        }
        unsafe {
            // SAFETY: index is checked to be within [0, len) above,
            // so the slot is initialized and ptr is non-null.
            let value = &*self.ptr.add(index);
            return Some(value);
        }
    }

    /// Grows the heap allocation to fit more elements.
    ///
    /// Called automatically by `push` when `len == cap`. The growth strategy
    /// doubles capacity on each call, which amortizes the cost of reallocation
    /// to O(1) per push on average.
    ///
    /// No-op for ZST since no allocation is ever needed.
    ///
    /// Panics with "Capacity Overflow" if doubling would exceed `usize::MAX`.
    ///
    /// If `alloc` or `realloc` returns a null pointer, `handle_alloc_error` is
    /// called. This aborts the process with an OOM message.
    fn grow(&mut self) {
        if std::mem::size_of::<T>() == 0 {
            return;
        }
        let old_cap = self.cap;
        let new_cap;
        if old_cap == 0 {
            new_cap = 1;
        } else {
            new_cap = old_cap.checked_mul(2).expect("Capacity Overflow");
        }
        unsafe {
            if old_cap == 0 {
                let new_layout = std::alloc::Layout::array::<T>(new_cap).unwrap();
                // SAFETY: layout is non-zero sized.
                let new_ptr = std::alloc::alloc(new_layout);
                if new_ptr.is_null() {
                    std::alloc::handle_alloc_error(new_layout);
                } else {
                    self.ptr = new_ptr as *mut T;
                }
            } else {
                let old_layout = std::alloc::Layout::array::<T>(old_cap).unwrap();
                let new_layout = std::alloc::Layout::array::<T>(new_cap).unwrap();
                // SAFETY: ptr was allocated with old_layout and is non-null.
                let new_ptr =
                    std::alloc::realloc(self.ptr as *mut u8, old_layout, new_layout.size());
                if new_ptr.is_null() {
                    std::alloc::handle_alloc_error(new_layout);
                } else {
                    self.ptr = new_ptr as *mut T;
                }
            }

            self.cap = new_cap;
        }
    }
}

/// Drops all initialized elements and frees the heap allocation.
///
/// Elements are dropped in order from index `0` to `len - 1` before the
/// heap block is deallocated. This ensures types like `String` or `Box<T>`
/// that own memory elsewhere have their destructors called correctly.
///
/// Skips deallocation entirely if `cap == 0` since no allocation was ever made.
impl<T> Drop for MyVec<T> {
    fn drop(&mut self) {
        if self.cap > 0 {
            // Run the destructor on every initialized element first.
            // This handles types like String or Box<T> that own heap memory
            // themselves. For types like i32 this is a no-op.
            // Must happen before dealloc - running drop_in_place after
            // freeing the block would be use-after-free.
            for i in 0..self.len() {
                unsafe {
                    // SAFETY: index is within [0, len) so the slot is
                    // initialized. drop_in_place runs T's destructor in
                    // place without moving the value out.
                    std::ptr::drop_in_place(self.ptr.add(i));
                }
            }
            let layout = std::alloc::Layout::array::<T>(self.cap).unwrap();
            if core::mem::size_of::<T>() == 0 {
                return;
            } else {
                unsafe {
                    // SAFETY: ptr was allocated with this layout and cap > 0
                    // guarantees a real allocation exists. All elements have
                    // already been dropped above.
                    std::alloc::dealloc(self.ptr as *mut u8, layout);
                }
            }
        }
    }
}

/// Enables indexing syntax `vec[i]`. Panics on out-of-bounds access.
impl<T> std::ops::Index<usize> for MyVec<T> {
    type Output = T;

    /// # Panics
    ///
    /// Panics if `index >= len`.
    fn index(&self, index: usize) -> &T {
        if index >= self.len() {
            panic!("Out of Bounds")
        }

        unsafe {
            // SAFETY: index is checked to be within [0, len) above.
            return &*self.ptr.add(index);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::MyVec;

    #[test]
    fn new_is_empty() {
        let v: MyVec<i32> = MyVec::new();
        assert_eq!(v.len(), 0);
        assert_eq!(v.cap(), 0);
    }

    #[test]
    fn push_increments_len() {
        let mut v: MyVec<i32> = MyVec::new();
        v.push(1);
        assert_eq!(v.len(), 1);
        v.push(2);
        assert_eq!(v.len(), 2);
    }

    #[test]
    fn capacity_doubles() {
        let mut v: MyVec<i32> = MyVec::new();
        v.push(1);
        assert_eq!(v.cap(), 1);
        v.push(2);
        assert_eq!(v.cap(), 2);
        v.push(3);
        assert_eq!(v.cap(), 4);
        v.push(4);
        assert_eq!(v.cap(), 4);
        v.push(5);
        assert_eq!(v.cap(), 8);
    }

    #[test]
    fn push_and_index() {
        let mut v: MyVec<i32> = MyVec::new();
        v.push(10);
        v.push(20);
        v.push(30);
        assert_eq!(v[0], 10);
        assert_eq!(v[1], 20);
        assert_eq!(v[2], 30);
    }

    #[test]
    fn pop_returns_in_order() {
        let mut v: MyVec<i32> = MyVec::new();
        v.push(1);
        v.push(2);
        v.push(3);
        assert_eq!(v.pop(), Some(3));
        assert_eq!(v.pop(), Some(2));
        assert_eq!(v.pop(), Some(1));
    }

    #[test]
    fn pop_on_empty_returns_none() {
        let mut v: MyVec<i32> = MyVec::new();
        assert_eq!(v.pop(), None);
    }

    #[test]
    fn pop_decrements_len() {
        let mut v: MyVec<i32> = MyVec::new();
        v.push(1);
        v.push(2);
        v.pop();
        assert_eq!(v.len(), 1);
    }

    #[test]
    fn get_returns_correct_value() {
        let mut v: MyVec<i32> = MyVec::new();
        v.push(100);
        v.push(200);
        assert_eq!(v.get(0), Some(&100));
        assert_eq!(v.get(1), Some(&200));
    }

    #[test]
    fn get_out_of_bounds_returns_none() {
        let mut v: MyVec<i32> = MyVec::new();
        v.push(1);
        assert_eq!(v.get(1), None);
        assert_eq!(v.get(99), None);
    }

    #[test]
    #[should_panic]
    fn index_out_of_bounds_panics() {
        let mut v: MyVec<i32> = MyVec::new();
        v.push(1);
        let _ = v[99];
    }

    #[test]
    fn drop_is_called() {
        struct D<'a>(&'a mut bool);
        impl Drop for D<'_> {
            fn drop(&mut self) {
                *self.0 = true;
            }
        }
        let mut dropped = false;
        {
            let mut v = MyVec::new();
            v.push(D(&mut dropped));
        }
        assert!(dropped);
    }

    #[test]
    fn zst_push_pop() {
        let mut v: MyVec<()> = MyVec::new();
        v.push(());
        v.push(());
        v.push(());
        assert_eq!(v.len(), 3);
        assert_eq!(v.pop(), Some(()));
        assert_eq!(v.len(), 2);
    }

    #[test]
    #[should_panic(expected = "Capacity Overflow")]
    fn growth_overflow_panics() {
        usize::MAX.checked_mul(2).expect("Capacity Overflow");
    }
}
