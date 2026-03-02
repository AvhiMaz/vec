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

    /// Appends `data` to the end of the vec.
    ///
    /// If the vec is full (`len == cap`), `grow` is called first to double
    /// capacity. The value is written into the heap slot at `len` using
    /// `ptr::write`, then `len` is incremented with `Release` ordering so
    /// concurrent readers see the new element only after it is fully written.
    pub fn append(&mut self, data: T) {
        if core::mem::size_of::<T>() != 0 && self.cap == self.len() {
            self.grow();
        }

        if core::mem::size_of::<T>() == 0 {
            std::mem::forget(data);
        } else {
            unsafe {
                // SAFETY: grow ensures ptr is valid for at least len+1 slots.
                // The slot at len is uninitialized so ptr::write is used to
                // avoid running Drop on whatever bytes are there.
                std::ptr::write(self.ptr.add(self.len()), data);
            }
            // Release pairs with Acquire in len() and get() - ensures the write
            // above is visible to any thread that observes the incremented len.
        }

        self.len.fetch_add(1, std::sync::atomic::Ordering::Release);
    }

    /// Grows the heap allocation to fit more elements.
    ///
    /// Doubles capacity on each call. No-op for ZST.
    ///
    /// Panics with "Capacity Overflow" if doubling would exceed `usize::MAX`.
    fn grow(&mut self) {
        if core::mem::size_of::<T>() == 0 {
            return;
        }
        let old_cap = self.cap;
        let new_cap = if old_cap == 0 {
            1
        } else {
            old_cap.checked_mul(2).expect("Capacity Overflow")
        };

        if old_cap == 0 {
            let layout = std::alloc::Layout::array::<T>(new_cap).unwrap();
            unsafe {
                // SAFETY: layout is non-zero sized.
                let new_ptr = std::alloc::alloc(layout);
                if new_ptr.is_null() {
                    std::alloc::handle_alloc_error(layout)
                } else {
                    self.ptr = new_ptr as *mut T;
                }
            }
        } else {
            let old_layout = std::alloc::Layout::array::<T>(old_cap).unwrap();
            let new_layout = std::alloc::Layout::array::<T>(new_cap).unwrap();

            unsafe {
                // SAFETY: ptr was allocated with old_layout and is non-null.
                let new_ptr =
                    std::alloc::realloc(self.ptr as *mut u8, old_layout, new_layout.size());

                if new_ptr.is_null() {
                    std::alloc::handle_alloc_error(new_layout)
                } else {
                    self.ptr = new_ptr as *mut T
                }
            }
        }
        self.cap = new_cap;
    }
}

impl<T> Default for AppendVec<T> {
    fn default() -> Self {
        Self::new()
    }
}
