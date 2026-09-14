use std::{mem::MaybeUninit, sync::Arc};

use super::Node;

/// Invariants:
///
/// - 0 <= length <= COUNT.
/// - All slots in range `0..length` are initialized.
/// - Slots in range `length..COUNT` may be initialized or uninitialized.
/// - Access to slots outside `0..length` is not allowed unless through
///   initialization APIs.
pub(super) struct NodeArray<T: ?Sized, const COUNT: usize> {
    pub data: [MaybeUninit<Arc<T>>; COUNT],
    pub length: usize,
}

impl<T: ?Sized, const COUNT: usize> NodeArray<T, COUNT> {
    #[inline(always)]
    pub fn new() -> Self {
        Self {
            data: unsafe { MaybeUninit::uninit().assume_init() },
            length: 0,
        }
    }

    #[inline(always)]
    pub fn length(&self) -> usize {
        self.length
    }

    /// Returns a reference to the first initialized element.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that:
    /// - `self.length > 0`.
    /// - The first element (`data[0]`) is initialized.
    ///
    /// Calling this function when the array is empty or the first slot is
    /// uninitialized results in undefined behavior.
    #[inline(always)]
    pub unsafe fn first_unchecked(&self) -> &Arc<T> {
        unsafe { self.data[0].assume_init_ref() }
    }

    #[inline(always)]
    pub fn first(&self) -> Option<&Arc<T>> {
        if self.length == 0 {
            return None;
        }

        Some(unsafe { &*self.first_unchecked() })
    }

    /// Returns a mutable reference to the first initialized element.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that:
    /// - `self.length > 0`.
    /// - The first element (`data[0]`) is initialized.
    ///
    /// Calling this function when the array is empty or the first slot is
    /// uninitialized results in undefined behavior.
    #[inline(always)]
    pub unsafe fn first_mut_unchecked(&mut self) -> &mut Arc<T> {
        unsafe { self.data[0].assume_init_mut() }
    }

    pub fn first_mut(&mut self) -> Option<&mut Arc<T>> {
        if self.length == 0 {
            return None;
        }

        Some(unsafe { self.first_mut_unchecked() })
    }

    /// Returns a reference to the last initialized element.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that:
    /// - `self.length > 0`.
    /// - Every element in the range `0..self.length` is initialized.
    ///
    /// Calling this function when the array is empty or when the last slot is
    /// uninitialized results in undefined behavior.
    #[inline(always)]
    pub unsafe fn last_unchecked(&self) -> &Arc<T> {
        unsafe { self.data[self.length - 1].assume_init_ref() }
    }

    #[inline(always)]
    pub fn last(&self) -> Option<&Arc<T>> {
        if self.length == 0 {
            return None;
        }

        Some(unsafe { &*self.last_unchecked() })
    }

    /// Returns a mutable reference to the last initialized element.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that:
    /// - `self.length > 0`.
    /// - Every element in the range `0..self.length` is initialized.
    ///
    /// Calling this function when the array is empty or when the last slot is
    /// uninitialized results in undefined behavior.
    #[inline(always)]
    pub unsafe fn last_mut_unchecked(&mut self) -> &mut Arc<T> {
        unsafe { self.data[self.length - 1].assume_init_mut() }
    }

    #[inline(always)]
    pub fn last_mut(&mut self) -> Option<&mut Arc<T>> {
        if self.length == 0 {
            return None;
        }

        Some(unsafe { self.last_mut_unchecked() })
    }

    /// Returns a reference to an initialized element at `index`.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that:
    /// - `index < self.length`.
    /// - The element at `index` is initialized.
    ///
    /// Calling this function with an invalid index or an uninitialized slot
    /// results in undefined behavior.
    #[inline(always)]
    pub unsafe fn get_unchecked(&self, index: usize) -> &Arc<T> {
        unsafe { self.data[index].assume_init_ref() }
    }

    #[inline(always)]
    pub fn get(&self, index: usize) -> Option<&Arc<T>> {
        if index >= self.length {
            return None;
        }

        Some(unsafe { self.get_unchecked(index) })
    }

    /// Returns a mutable reference to an initialized element at `index`.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that:
    /// - `index < self.length`.
    /// - The element at `index` is initialized.
    ///
    /// Calling this function with an invalid index or an uninitialized slot
    /// results in undefined behavior.
    #[inline(always)]
    pub unsafe fn get_mut_unchecked(&mut self, index: usize) -> &mut Arc<T> {
        unsafe { self.data[index].assume_init_mut() }
    }

    #[inline(always)]
    pub fn get_mut(&mut self, index: usize) -> Option<&mut Arc<T>> {
        if index >= self.length {
            return None;
        }

        Some(unsafe { self.get_mut_unchecked(index) })
    }

    /// # Safety
    ///
    /// The caller must guarantee that:
    /// - `index < self.length`.
    /// - The slot at `index` is initialized.
    ///
    /// The function reads the existing value using `assume_init_read()` and
    /// replaces it with `value`. Calling this function on an uninitialized slot
    /// results in undefined behavior.
    #[inline(always)]
    pub unsafe fn replace_unchecked(&mut self, index: usize, value: Arc<T>) -> Arc<T> {
        let old = unsafe { self.data[index].assume_init_read() };

        self.data[index].write(value);

        old
    }

    #[inline(always)]
    pub fn replace(&mut self, index: usize, value: Arc<T>) -> Option<Arc<T>> {
        if index >= self.length {
            return None;
        }

        unsafe { Some(self.replace_unchecked(index, value)) }
    }

    /// Removes and returns the last initialized element.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that:
    /// - `self.length > 0`.
    /// - The last element (`data[self.length - 1]`) is initialized.
    ///
    /// Calling this function when the array is empty or the last slot is
    /// uninitialized results in undefined behavior.
    #[inline(always)]
    pub unsafe fn pop_unchecked(&mut self) -> Arc<T> {
        self.length -= 1;

        unsafe { self.data[self.length].assume_init_read() }
    }

    #[inline(always)]
    pub fn pop(&mut self) -> Option<Arc<T>> {
        if self.length == 0 {
            return None;
        }

        Some(unsafe { self.pop_unchecked() })
    }

    /// Appends a value without checking capacity.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that:
    /// - `self.length < COUNT`.
    /// - Every element in the range `0..self.length` is initialized.
    /// - The resulting array satisfies the `NodeArray` initialization invariant.
    ///
    /// Calling this function when the array is full results in writing beyond
    /// the valid storage range and causes undefined behavior.
    #[inline(always)]
    pub unsafe fn push_unchecked(&mut self, value: Arc<T>) {
        self.data[self.length].write(value);
        self.length += 1;
    }

    #[inline(always)]
    pub fn push(&mut self, value: Arc<T>) -> bool {
        if self.length == COUNT {
            return false;
        }

        unsafe { self.push_unchecked(value) };
        true
    }

    #[inline(always)]
    pub fn as_slice(&self) -> &[Arc<T>] {
        unsafe { std::slice::from_raw_parts(self.data.as_ptr() as *const Arc<T>, self.length) }
    }

    #[inline(always)]
    pub fn as_mut_slice(&mut self) -> &mut [Arc<T>] {
        unsafe {
            std::slice::from_raw_parts_mut(self.data.as_mut_ptr() as *mut Arc<T>, self.length)
        }
    }

    #[inline(always)]
    pub fn validate(&self) -> Result<(), String> {
        if self.length > COUNT {
            return Err(format!(
                "length must be in the range 0..={COUNT}, got {}",
                self.length
            ));
        }

        Ok(())
    }

    #[inline(always)]
    pub fn truncate(&mut self, len: usize) {
        while self.length > len {
            self.length -= 1;

            unsafe {
                self.data[self.length].assume_init_drop();
            }
        }
    }
}

impl<T: ?Sized, const COUNT: usize> Clone for NodeArray<T, COUNT> {
    fn clone(&self) -> Self {
        let mut result = Self::new();

        for index in 0..self.length {
            result.data[index].write(unsafe { self.data[index].assume_init_ref().clone() });
        }

        result.length = self.length;

        result
    }
}

impl<T: ?Sized, const COUNT: usize> Drop for NodeArray<T, COUNT> {
    fn drop(&mut self) {
        for i in 0..self.length {
            unsafe {
                self.data[i].assume_init_drop();
            }
        }
    }
}

/// BranchNode invariants:
///
/// - 0 < children.length() <= COUNT.
/// - Every child exists.
/// - Every child.level == self.level - 1.
/// - No gaps exist.
/// - All children except the last are full.
pub(super) struct BranchNode<T: ?Sized, const COUNT: usize> {
    pub children: NodeArray<Node<T, COUNT>, COUNT>,
    pub level: usize,
}

/// LeafNode invariants:
///
/// - 0 < values.length() <= COUNT.
/// - All stored values are initialized.
/// - No gaps exist.
pub(super) struct LeafNode<T: ?Sized, const COUNT: usize> {
    pub values: NodeArray<T, COUNT>,
}

impl<T: ?Sized, const COUNT: usize> BranchNode<T, COUNT> {
    #[inline(always)]
    pub fn new(children: NodeArray<Node<T, COUNT>, COUNT>, level: usize) -> Option<Self> {
        let new = Self::new_raw(children, level);
        new.validate().ok()?;
        Some(new)
    }

    /// Creates a branch node without validating invariants.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that:
    ///
    /// - `children.length()` is in the range `1..=COUNT`.
    /// - Every element in `children` is initialized.
    /// - Every child node satisfies its own invariants.
    /// - Every child node has `level == level - 1`.
    /// - No gaps exist in `children`.
    /// - `level > 0`.
    ///
    /// Violating these requirements results in a `BranchNode` that does not
    /// satisfy its invariants and may cause undefined behavior when unchecked
    /// operations rely on those invariants.
    #[inline(always)]
    pub unsafe fn new_unchecked(children: NodeArray<Node<T, COUNT>, COUNT>, level: usize) -> Self {
        Self::new_raw(children, level)
    }

    #[inline(always)]
    pub fn new_raw(children: NodeArray<Node<T, COUNT>, COUNT>, level: usize) -> Self {
        Self { children, level }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.level == 0 {
            return Err("branch node level must be greater than 0".to_string());
        }

        self.children.validate()?;

        if self.children.length() == 0 {
            return Err("branch node must contain at least one child".to_string());
        }

        for index in 0..self.children.length() {
            let child = unsafe { self.children.get_unchecked(index) };

            if child.level() != self.level - 1 {
                return Err(format!(
                    "unbalanced tree: child level {}, expected {}",
                    child.level(),
                    self.level - 1
                ));
            }

            child.validate_node()?;
        }

        Ok(())
    }

    #[inline(always)]
    pub fn push_child(&mut self, child: Arc<Node<T, COUNT>>) {
        self.children.push(child);
    }

    #[inline(always)]
    pub fn get(&self, index: usize) -> Option<&Arc<Node<T, { COUNT }>>> {
        self.children.get(index)
    }

    #[inline(always)]
    pub fn child_span(&self) -> usize {
        COUNT.pow((self.level) as u32)
    }

    #[inline(always)]
    pub fn span(&self) -> usize {
        self.child_span() * self.children.length
    }

    #[inline(always)]
    pub fn max_capacity(&self) -> usize {
        self.child_span() * COUNT
    }
}

impl<T: ?Sized, const COUNT: usize> LeafNode<T, COUNT> {
    const LEVEL: usize = 0;
    const CHILD_SPAN: usize = 1;

    #[inline(always)]
    pub fn new(values: NodeArray<T, COUNT>) -> Option<Self> {
        let new = Self::new_raw(values);
        new.validate().ok()?;
        Some(new)
    }

    /// Creates a leaf node without validating its invariants.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that:
    ///
    /// - `0 < values.length() <= COUNT`.
    /// - Every element in the initialized range `0..values.length()` contains a
    ///   valid initialized value.
    /// - The initialized elements are contiguous and there are no gaps.
    /// - No operation requiring a valid `LeafNode` invariant is performed if these
    ///   conditions are not satisfied.
    ///
    /// Violating these requirements may result in undefined behavior when the node
    /// is accessed through methods that assume a valid leaf node.
    #[inline(always)]
    pub unsafe fn new_unchecked(values: NodeArray<T, COUNT>) -> Self {
        Self::new_raw(values)
    }

    #[inline(always)]
    pub fn new_raw(values: NodeArray<T, COUNT>) -> Self {
        Self { values }
    }

    #[inline(always)]
    pub fn validate(&self) -> Result<(), String> {
        self.values.validate()?;

        if self.values.length() == 0 {
            return Err("leaf node must contain at least one value".to_string());
        }

        Ok(())
    }

    #[inline(always)]
    pub fn get(&self, index: usize) -> Option<&Arc<T>> {
        self.values.get(index)
    }

    #[inline(always)]
    pub fn span(&self) -> usize {
        self.values.length
    }
}

impl<T: ?Sized, const CAPACITY: usize> Clone for BranchNode<T, CAPACITY> {
    fn clone(&self) -> Self {
        Self {
            children: self.children.clone(),
            level: self.level,
        }
    }
}

impl<T: ?Sized, const CAPACITY: usize> Clone for LeafNode<T, CAPACITY> {
    fn clone(&self) -> Self {
        Self {
            values: self.values.clone(),
        }
    }
}
