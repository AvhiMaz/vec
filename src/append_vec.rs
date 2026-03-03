/// An append-only, concurrent-read vec built on raw pointers and atomics.
///
/// On the stack, `AppendVec<T>` is three words:
///
/// - `ptr` - raw pointer to a contiguous block of heap memory holding `T` values
/// - `cap` - total number of `T` slots the block can hold before reallocation
/// - `len` - atomic counter of elements written, safe to read across threads
///
/// `len` is an `AtomicUsize` so readers can call `get` and `len` concurrently
/// while a single writer is appending. Only `append` requires `&mut self`,
/// enforcing a single writer at the type level.
pub struct AppendVec<T> {
    /// Raw pointer to the start of the heap allocation.
    /// Null when `cap == 0`.
    pub ptr: *mut T,
    /// Total number of `T` slots available in the current allocation.
    cap: usize,
    /// Number of elements appended so far.
    /// Written with `Release` on append, read with `Acquire` on get/len.
    len: std::sync::atomic::AtomicUsize,
}

impl<T> AppendVec<T> {
    /// Returns the number of elements currently stored.
    ///
    /// Uses `Acquire` ordering so any element written before the corresponding
    /// `Release` in `append` is visible to the caller.
    pub fn len(&self) -> usize {
        self.len.load(std::sync::atomic::Ordering::Acquire)
    }

    /// Returns `true` if no elements have been appended yet.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the total number of elements the vec can hold without reallocating.
    pub fn cap(&self) -> usize {
        self.cap
    }

    /// Creates a new, empty `AppendVec<T>` without allocating any heap memory.
    pub fn new() -> Self {
        AppendVec {
            ptr: std::ptr::null_mut(),
            len: std::sync::atomic::AtomicUsize::new(0),
            cap: 0,
        }
    }

    /// Returns a shared reference to the element at `index`, or `None` if
    /// `index` is out of bounds.
    ///
    /// `len` is loaded with `Acquire` ordering first, then the index is
    /// bounds-checked against it. This pairs with the `Release` store in
    /// `append`, guaranteeing that any slot visible through this method is
    /// fully written - readers can never observe partially initialized data.
    ///
    /// For ZST, no memory is accessed. A dangling non-null pointer is used
    /// since a ZST reference requires only alignment, not a real address.
    pub fn get(&self, index: usize) -> Option<&T> {
        let len = self.len.load(std::sync::atomic::Ordering::Acquire);

        if index >= len {
            return None;
        }

        if core::mem::size_of::<T>() == 0 {
            unsafe {
                // SAFETY: T is a ZST so no memory is accessed when forming
                // this reference. NonNull::dangling() provides a non-null
                // aligned pointer which is all a ZST reference requires.
                Some(&*std::ptr::NonNull::<T>::dangling().as_ptr())
            }
        } else {
            unsafe {
                // SAFETY: index < len (Acquire) guarantees the slot is fully
                // initialized. ptr is non-null and valid for `cap` slots.
                Some(&*self.ptr.add(index))
            }
        }
    }

    /// Appends `data` to the end of the vec.
    ///
    /// Panics if the vec is full (`len == cap`). The vec is fixed-capacity -
    /// it never reallocates, which keeps existing pointers held by concurrent
    /// readers valid for the lifetime of the vec.
    ///
    /// The value is written into the heap slot at `len` using `ptr::write`,
    /// then `len` is incremented with `Release` ordering so concurrent readers
    /// see the new element only after it is fully written.
    ///
    /// For ZST, no memory is written. `data` is forgotten and only `len` is
    /// incremented - ZSTs carry no bytes so capacity is irrelevant.
    pub fn append(&mut self, data: T) {
        if core::mem::size_of::<T>() != 0 && self.cap == self.len() {
            panic!("Out of Bounds")
        }

        if core::mem::size_of::<T>() == 0 {
            std::mem::forget(data);
        } else {
            unsafe {
                // SAFETY: with_capacity ensures ptr is valid for `cap` slots.
                // We checked len < cap above, so this slot is in bounds.
                // The slot is uninitialized so ptr::write is used to avoid
                // running Drop on whatever bytes happen to be there.
                std::ptr::write(self.ptr.add(self.len()), data);
            }
            // Release pairs with Acquire in len() and get() - ensures the
            // write above is visible to any thread that observes the
            // incremented len.
        }

        self.len.fetch_add(1, std::sync::atomic::Ordering::Release);
    }

    /// Creates a new `AppendVec<T>` with `cap` pre-allocated slots.
    ///
    /// The full capacity is allocated upfront and never changes. This is
    /// intentional: reallocating would move the heap block to a new address,
    /// invalidating any pointers held by concurrent readers.
    ///
    /// Returns an empty `AppendVec` (same as `new`) when `cap == 0` or `T`
    /// is a ZST, since neither case requires heap memory.
    pub fn with_capacity(cap: usize) -> Self {
        if cap == 0 || core::mem::size_of::<T>() == 0 {
            Self::new()
        } else {
            unsafe {
                let layout = std::alloc::Layout::array::<T>(cap).unwrap();
                let new_ptr = std::alloc::alloc(layout);

                if new_ptr.is_null() {
                    std::alloc::handle_alloc_error(layout)
                }

                AppendVec {
                    ptr: new_ptr as *mut T,
                    cap,
                    len: std::sync::atomic::AtomicUsize::new(0),
                }
            }
        }
    }
}

/// Drops all initialized elements and frees the heap allocation.
///
/// At drop time we hold `&mut self` so no concurrent readers exist.
/// `get_mut()` is used instead of an atomic load - no ordering overhead
/// is needed when we have exclusive access.
///
/// Elements are dropped in reverse order before `dealloc` so types like
/// `String` or `Box<T>` have their destructors called before the backing
/// memory is freed. Dealloc is skipped when `cap == 0` (no allocation
/// was ever made) or T is a ZST (no heap was ever touched).
impl<T> Drop for AppendVec<T> {
    fn drop(&mut self) {
        let len = *self.len.get_mut();
        let ptr = if core::mem::size_of::<T>() == 0 {
            // SAFETY: T is a ZST - drop_in_place never dereferences the
            // pointer, so any aligned non-null address is valid here.
            std::ptr::NonNull::dangling().as_ptr()
        } else {
            self.ptr
        };
        for i in (0..len).rev() {
            unsafe {
                // SAFETY: index is within [0, len) so the slot is
                // initialized. drop_in_place runs T's destructor in place
                // without moving the value out.
                std::ptr::drop_in_place(ptr.add(i));
            }
        }
        if self.cap > 0 && core::mem::size_of::<T>() != 0 {
            let layout = std::alloc::Layout::array::<T>(self.cap).unwrap();
            unsafe {
                // SAFETY: ptr was allocated with this layout and cap > 0
                // guarantees a real allocation exists. All elements have
                // already been dropped above.
                std::alloc::dealloc(self.ptr as *mut u8, layout);
            }
        }
    }
}

impl<T> Default for AppendVec<T> {
    fn default() -> Self {
        Self::new()
    }
}
