pub struct AppendVec<T> {
    pub ptr: *mut T,
    cap: usize,
    len: std::sync::atomic::AtomicUsize,
}

impl<T> AppendVec<T> {
    pub fn new() -> Self {
        AppendVec {
            ptr: std::ptr::null_mut(),
            len: std::sync::atomic::AtomicUsize::new(0),
            cap: 0,
        }
    }
}

impl<T> Default for AppendVec<T> {
    fn default() -> Self {
        Self::new()
    }
}
