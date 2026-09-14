mod iter;
mod node_kind;
mod test;

use std::{
    fmt::{Debug, Formatter},
    mem::MaybeUninit,
    sync::Arc,
};
use std::fmt::Display;
use iter::Iter;
use node_kind::{BranchNode, LeafNode, NodeArray};

const NODE_CAPACITY: usize = 32; // todo: 4 -> 32

pub type PersistentVector<T: ?Sized> = InnerPersistentVector<T, NODE_CAPACITY>;

/// Invariants:
///
/// - `size == 0` if and only if `root.is_none()` and `tail.is_none()`.
/// - `size > 0` if and only if either `root.is_some()` or `tail.is_some()`.
/// - `height == 0` if and only if `root.is_none()`.
/// - If `root.is_some()`, then `height == root.level()`.
/// - Every reachable node satisfies `Node`'s invariants.
/// - Every leaf node inside `root` is at the same depth (`height`) from the root.
pub struct InnerPersistentVector<T: ?Sized, const CAPACITY: usize> {
    root: Option<Arc<Node<T, CAPACITY>>>,
    size: usize,
    height: usize,
}

/// Invariants:
///
/// - `COUNT >= 2`.
/// - `level == 0` if and only if this is a leaf node.
/// - `level > 0` if and only if this is a branch node.
/// - Every child of a branch node has `level == self.level - 1`.
/// - Branch nodes always contain at least one child.
/// - Leaf nodes always contain at least one value.
/// - Empty nodes are never represented.
///
/// - Elements are stored in index order without gaps.
///
/// - Branch nodes:
///   - `children.length() > 0`.
///   - Every child exists within `children.length()`.
///   - Every child satisfies the level invariant.
///   - Children after `children.length()` are uninitialized.
///
/// - Leaf nodes:
///   - `values.length() > 0`.
///   - All values within `values.length()` are initialized.
///   - Values are stored contiguously.
///
/// Valid:
/// - Branch node with children `[A, B, C]`.
/// - Leaf node with values `[a, b, c]`.
///
/// Invalid:
/// - Branch node with zero children.
/// - Leaf node with zero values.
/// - Child with an incorrect level.
/// - Any internal gap.
///
/// - Non-empty nodes only.
/// - Level ordering is preserved.
/// - Children are stored in index order.
/// - All children except the last child are full.
/// - Only the last child may be partially filled.
enum Node<T: ?Sized, const COUNT: usize> {
    Branch(BranchNode<T, COUNT>),
    Leaf(LeafNode<T, COUNT>),
}

impl<T: ?Sized, const CAPACITY: usize> InnerPersistentVector<T, CAPACITY> {
    #[inline(always)]
    pub fn new() -> Self {
        Self {
            root: None,
            size: 0,
            height: 0,
        }
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.size
    }

    #[inline(always)]
    pub fn height(&self) -> usize {
        self.height
    }

    #[inline(always)]
    pub fn capacity(&self) -> usize {
        CAPACITY.pow(self.height as u32)
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

    #[inline(always)]
    pub fn first(&self) -> Option<Arc<T>> {
        self.get(0)
    }

    #[inline(always)]
    pub fn last(&self) -> Option<Arc<T>> {
        if self.is_empty() {
            return None;
        }

        self.get(self.size - 1)
    }

    pub fn get(&self, index: usize) -> Option<Arc<T>> {
        if index >= self.size {
            return None;
        }

        self.root.as_ref()?.get(index)
    }

    pub fn pop(&self) -> Option<Self> {
        if self.size == 0 {
            return None;
        }

        let root = self.root.as_ref()?;

        let mut result = Self::new();

        match root.pop() {
            PopResult::Empty => {
                return Some(result);
            }

            PopResult::Update(root) => {
                result.root = Some(root);
                result.size = self.size - 1;
                result.height = result.root.as_ref().map(|x| x.level()).unwrap_or(0);
            }
        }

        Some(result)
    }

    pub fn truncate(&self, len: usize) -> Self {
        assert!(len <= self.size);

        if len == 0 {
            return Self::new();
        }

        if len == self.size {
            return self.clone();
        }

        let root = self.root.as_ref().unwrap();

        match root.truncate(len) {
            TruncateResult::Empty => Self::new(),

            TruncateResult::Node(root) => Self {
                height: root.level(),
                root: Some(root),
                size: len,
            },
        }
    }

    pub fn clear(&self) -> Self {
        Self::new()
    }

    pub fn push(&self, value: Arc<T>) -> Self {
        let mut result = self.clone();

        match result.root.take() {
            None => {
                let mut values = NodeArray::new();

                unsafe {
                    values.push_unchecked(value);
                }

                let leaf = unsafe { LeafNode::new_unchecked(values) };

                result.root = Some(Arc::new(unsafe { Node::new_unchecked(Node::Leaf(leaf)) }));

                result.height = 0;
                result.size = 1;

                result
            }

            Some(root) => {
                match root.try_push(value.clone()) {
                    PushResult::Update(node) => {
                        result.root = Some(node);
                    }

                    PushResult::Full => {
                        let new_child = Node::singleton(result.height, value);

                        let mut children = NodeArray::new();

                        unsafe {
                            children.push_unchecked(root);
                            children.push_unchecked(new_child);
                        }

                        let branch =
                            unsafe { BranchNode::new_unchecked(children, result.height + 1) };

                        result.root = Some(Arc::new(unsafe {
                            Node::new_unchecked(Node::Branch(branch))
                        }));

                        result.height += 1;
                    }
                }

                result.size += 1;
                result
            }
        }
    }

    pub fn append(&self, other: &Self) -> Self {
        let mut result = self.clone();

        for value in other.iter() {
            result = result.push(value);
        }

        result
    }

    pub fn remove(&self, index: usize) -> Self {
        assert!(index < self.size);

        let mut result = Self::new();

        for (i, value) in self.iter().enumerate() {
            if i != index {
                result = result.push(value);
            }
        }

        result
    }

    pub fn iter(&self) -> Iter<T, CAPACITY> {
        Iter::new(self.root.clone(), self.size)
    }

    #[inline(always)]
    pub fn contains(&self, value: &T) -> bool
    where
        T: PartialEq,
    {
        self.iter().any(|item| item.as_ref() == value)
    }

    pub fn sort(&self) -> Self
    where
        T: Ord,
    {
        let mut values: Vec<Arc<T>> = self.iter().collect();

        values.sort_by(|a, b| a.as_ref().cmp(b.as_ref()));

        let mut result = Self::new();

        for value in values {
            result = result.push(value);
        }

        result
    }

    pub fn sort_unstable(&self) -> Self
    where
        T: Ord,
    {
        let mut values: Vec<Arc<T>> = self.iter().collect();

        values.sort_unstable_by(|a, b| a.as_ref().cmp(b.as_ref()));

        let mut result = Self::new();

        for value in values {
            result = result.push(value);
        }

        result
    }

    pub fn sort_by<F>(&self, mut compare: F) -> Self
    where
        F: FnMut(&T, &T) -> std::cmp::Ordering,
    {
        let mut values: Vec<Arc<T>> = self.iter().collect();

        values.sort_by(|a, b| compare(a.as_ref(), b.as_ref()));

        let mut result = Self::new();

        for value in values {
            result = result.push(value);
        }

        result
    }

    pub fn sort_unstable_by<F>(&self, mut compare: F) -> Self
    where
        F: FnMut(&T, &T) -> std::cmp::Ordering,
    {
        let mut values: Vec<Arc<T>> = self.iter().collect();

        values.sort_unstable_by(|a, b| compare(a.as_ref(), b.as_ref()));

        let mut result = Self::new();

        for value in values {
            result = result.push(value);
        }

        result
    }

    pub fn sort_by_key<K, F>(&self, mut f: F) -> Self
    where
        F: FnMut(&T) -> K,
        K: Ord,
    {
        let mut values: Vec<Arc<T>> = self.iter().collect();

        values.sort_by_key(|value| f(value.as_ref()));

        let mut result = Self::new();

        for value in values {
            result = result.push(value);
        }

        result
    }

    pub fn sort_unstable_by_key<K, F>(&self, mut f: F) -> Self
    where
        F: FnMut(&T) -> K,
        K: Ord,
    {
        let mut values: Vec<Arc<T>> = self.iter().collect();

        values.sort_unstable_by_key(|value| f(value.as_ref()));

        let mut result = Self::new();

        for value in values {
            result = result.push(value);
        }

        result
    }

    pub fn sort_by_cached_key<K, F>(&self, mut f: F) -> Self
    where
        F: FnMut(&T) -> K,
        K: Ord,
    {
        let mut values: Vec<(K, Arc<T>)> = self
            .iter()
            .map(|value| (f(value.as_ref()), value))
            .collect();

        values.sort_by(|a, b| a.0.cmp(&b.0));

        let mut result = Self::new();

        for (_, value) in values {
            result = result.push(value);
        }

        result
    }

    pub fn sort_unstable_by_cached_key<K, F>(&self, mut f: F) -> Self
    where
        F: FnMut(&T) -> K,
        K: Ord,
    {
        let mut values: Vec<(K, Arc<T>)> = self
            .iter()
            .map(|value| (f(value.as_ref()), value))
            .collect();

        values.sort_unstable_by(|a, b| a.0.cmp(&b.0));

        let mut result = Self::new();

        for (_, value) in values {
            result = result.push(value);
        }

        result
    }
}

impl<T: Sized, const CAPACITY: usize> InnerPersistentVector<T, CAPACITY> {
    pub fn set(&self, index: usize, value: T) -> Result<Self, String> {
        match self.root {
            None if index == 0 => {
                let mut persistent_vector = Self::new();
                persistent_vector.root = Some(Node::singleton(0, Arc::new(value)));
                persistent_vector.size = 1;
                Ok(persistent_vector)
            }
            Some(ref root) if index < self.size => {
                let mut persistent_vector = self.clone();
                persistent_vector.root = root.set(index, Arc::new(value));
                Ok(persistent_vector)
            }
            _ => Err("invalid index. check to index".to_string()),
        }
    }

    pub fn update<F>(&self, index: usize, f: F) -> Result<Self, String>
    where
        F: FnOnce(&T) -> T,
    {
        let old = self.get(index).ok_or_else(|| "invalid index".to_string())?;

        let new_value = Arc::new(f(&old));

        let root = self
            .root
            .as_ref()
            .ok_or_else(|| "empty vector".to_string())?
            .set(index, new_value)
            .ok_or_else(|| "set failed".to_string())?;

        Ok(Self {
            root: Some(root),
            size: self.size,
            height: self.height,
        })
    }

    pub fn insert(&self, index: usize, value: T) -> Self {
        assert!(index <= self.size);

        let mut result = Self::new();
        let value = Arc::new(value);

        for (i, current) in self.iter().enumerate() {
            if i == index {
                result = result.push(value.clone());
            }

            result = result.push(current);
        }

        if index == self.size {
            result = result.push(value);
        }

        result
    }

    pub fn extend<I>(&self, iter: I) -> Self
    where
        I: IntoIterator<Item = T>,
    {
        let mut result = self.clone();

        for value in iter {
            result = result.push(Arc::new(value));
        }

        result
    }
}

impl<T: ?Sized, const COUNT: usize> Node<T, COUNT> {
    fn new(kind: Self) -> Result<Self, String> {
        Self::validate(&kind)?;

        Ok(Self::new_raw(kind))
    }

    /// Creates a node without validating invariants.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that:
    /// - `0 < length <= COUNT`.
    /// - `level == 0` if and only if `kind` is `Leaf`.
    /// - `level > 0` if and only if `kind` is `Branch`.
    /// - Branch children all have `level == level - 1`.
    /// - Leaf and branch arrays satisfy the `Some...Some,None...None` layout.
    unsafe fn new_unchecked(kind: Self) -> Self {
        Self::new_raw(kind)
    }

    #[inline]
    fn new_raw(kind: Self) -> Self {
        kind
    }

    fn validate(&self) -> Result<(), String> {
        match self {
            Node::Leaf(leaf) => leaf.validate(),

            Node::Branch(branch) => branch.validate(),
        }
    }

    fn validate_node(&self) -> Result<(), String> {
        Self::validate(&self)
    }

    fn singleton(level: usize, value: Arc<T>) -> Arc<Self> {
        if level == 0 {
            let mut leaf = NodeArray::new();
            leaf.push(value);
            let leaf = unsafe { LeafNode::new_unchecked(leaf) };
            return Arc::new(unsafe { Self::new_unchecked(Self::Leaf(leaf)) });
        }

        let mut node_array = NodeArray::new();
        node_array.push(Self::singleton(level - 1, value));
        let branch = unsafe { BranchNode::new_unchecked(node_array, level) };

        Arc::new(unsafe { Self::new_unchecked(Self::Branch(branch)) })
    }

    #[inline(always)]
    fn length(&self) -> usize {
        match self {
            Self::Branch(branch) => branch.children.length,
            Self::Leaf(leaf) => leaf.values.length,
        }
    }

    #[inline(always)]
    fn length_mut(&mut self) -> &mut usize {
        match self {
            Self::Branch(branch) => &mut branch.children.length,
            Self::Leaf(leaf) => &mut leaf.values.length,
        }
    }

    #[inline(always)]
    fn level(&self) -> usize {
        match &self {
            Self::Branch(branch) => branch.level,
            Self::Leaf(_) => 0,
        }
    }

    #[inline(always)]
    fn span(&self) -> usize {
        match self {
            Self::Branch(branch) => branch.span(),
            Self::Leaf(leaf) => leaf.span(),
        }
    }

    #[inline(always)]
    fn child_span(&self) -> usize {
        match self {
            Self::Branch(branch) => branch.child_span(),
            Self::Leaf(_) => 1,
        }
    }

    #[inline(always)]
    fn max_capacity(&self) -> usize {
        COUNT.pow(self.level() as u32)
    }

    fn is_leaf(&self) -> bool {
        matches!(self, Self::Leaf(_))
    }

    fn is_branch(&self) -> bool {
        !self.is_leaf()
    }

    #[inline(always)]
    fn leaf(&self) -> &LeafNode<T, COUNT> {
        let Self::Leaf(items) = self else {
            unreachable!("Node invariant violated");
        };
        items
    }

    #[inline(always)]
    fn leaf_mut(&mut self) -> &mut LeafNode<T, COUNT> {
        let Self::Leaf(items) = self else {
            unreachable!("Node invariant violated");
        };
        items
    }

    #[inline(always)]
    fn branch(&self) -> &BranchNode<T, COUNT> {
        let Self::Branch(children) = self else {
            unreachable!("Node invariant violated");
        };
        children
    }

    #[inline(always)]
    fn branch_mut(&mut self) -> &mut BranchNode<T, COUNT> {
        let Self::Branch(children) = self else {
            unreachable!("Node invariant violated");
        };
        children
    }

    /// Returns a shared reference to the inner leaf items without checking the node type.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `self` is [`Self::Leaf`].
    ///
    /// Calling this method on a branch node violates memory safety and results in
    /// Undefined Behavior, as it misinterprets branch node memory layout as leaf items.
    #[inline(always)]
    unsafe fn leaf_unchecked(&self) -> &LeafNode<T, COUNT> {
        match self {
            Self::Leaf(items) => items,
            _ => unsafe { std::hint::unreachable_unchecked() },
        }
    }

    /// Returns a mutable reference to the inner leaf items without checking the node type.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `self` is [`Self::Leaf`].
    ///
    /// Calling this method on a branch node violates memory safety and results in
    /// Undefined Behavior, as it misinterprets branch node memory layout as leaf items.
    #[inline(always)]
    unsafe fn leaf_mut_unchecked(&mut self) -> &mut LeafNode<T, COUNT> {
        match self {
            Self::Leaf(items) => items,
            _ => unsafe { std::hint::unreachable_unchecked() },
        }
    }

    /// Returns a shared reference to the inner branch children without checking the node type.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `self` is [`Self::Branch`].
    ///
    /// Calling this method on a leaf node violates memory safety and results in
    /// Undefined Behavior, as it misinterprets leaf node memory layout as branch children.
    #[inline(always)]
    unsafe fn branch_unchecked(&self) -> &BranchNode<T, COUNT> {
        match self {
            Self::Branch(items) => items,
            _ => unsafe { std::hint::unreachable_unchecked() },
        }
    }

    /// Returns a mutable reference to the inner branch children without checking the node type.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `self` is [`Self::Branch`].
    ///
    /// Calling this method on a leaf node violates memory safety and results in
    /// Undefined Behavior, as it misinterprets leaf node memory layout as branch children.
    #[inline(always)]
    unsafe fn branch_mut_unchecked(&mut self) -> &mut BranchNode<T, COUNT> {
        match self {
            Self::Branch(items) => items,
            _ => unsafe { std::hint::unreachable_unchecked() },
        }
    }

    #[inline(always)]
    fn try_leaf(&self) -> Option<&LeafNode<T, COUNT>> {
        let Self::Leaf(items) = self else {
            return None;
        };
        Some(items)
    }

    #[inline(always)]
    fn try_branch(&self) -> Option<&BranchNode<T, COUNT>> {
        let Self::Branch(children) = self else {
            return None;
        };
        Some(children)
    }

    fn replace_child(
        &self,
        index: usize,
        child: Arc<Node<T, COUNT>>,
    ) -> ReplaceChildResult<T, COUNT> {
        if index >= self.length() {
            return ReplaceChildResult::InvalidIndex;
        }
        if !self.is_branch() {
            return ReplaceChildResult::NotBranch;
        }
        if self.level() - 1 != child.level() {
            return ReplaceChildResult::InvalidLevel;
        }

        let mut node = self.clone();

        unsafe {
            node.branch_mut().children.replace_unchecked(index, child);
        }

        ReplaceChildResult::ReplacedNode(Arc::new(node))
    }

    fn append_child(&self, child: Arc<Node<T, COUNT>>) -> AppendChildResult<T, COUNT> {
        if self.length() == COUNT {
            return AppendChildResult::Full;
        }
        if !self.is_branch() {
            return AppendChildResult::NotBranch;
        }
        if self.level() - 1 != child.level() {
            return AppendChildResult::InvalidLevel;
        }

        let mut node = self.clone();
        node.branch_mut().children.push(child);
        AppendChildResult::AppendNode(Arc::new(node))
    }

    fn get(&self, index: usize) -> Option<Arc<T>> {
        match self {
            Self::Leaf(leaf) => leaf.get(index).cloned(),
            Self::Branch(_) => {
                let child_span = self.child_span();

                let child = index / child_span;
                let rest = index % child_span;

                self.branch().get(child)?.get(rest)
            }
        }
    }

    fn set(&self, index: usize, value: Arc<T>) -> Option<Arc<Self>> {
        match self {
            Self::Leaf(leaf) => {
                let mut node = self.clone();

                node.leaf_mut().values.replace(index, value)?;

                Some(Arc::new(node))
            }

            Self::Branch(branch) => {
                let child = index / self.child_span();
                let rest = index % self.child_span();

                let target = branch.children.get(child)?;

                let new_child = target.set(rest, value)?;

                let mut node = self.clone();

                let branch = node.branch_mut();

                // child는 이미 존재하는 index이므로 안전
                branch.children.replace(child, new_child);

                Some(Arc::new(node))
            }
        }
    }

    fn try_push(&self, value: Arc<T>) -> PushResult<T, COUNT> {
        match self {
            Self::Leaf(leaf) => {
                if self.length() == COUNT {
                    PushResult::Full
                } else {
                    // Leaf일 때 set 대신 length 위치에 직접 인서트 후 length + 1 필요
                    let mut node = self.clone();
                    node.leaf_mut().values.push(value);
                    PushResult::Update(Arc::new(node))
                }
            }
            Self::Branch(children) => {
                let last_child_idx = self.length() - 1;
                let last_child = self.branch().children.get(last_child_idx).clone().unwrap();
                let result = last_child.try_push(value.clone());

                match result {
                    PushResult::Update(updated_child) => {
                        // 하위 노드가 업데이트되었으면 부모 노드도 새 자식을 가리키도록 갱신
                        let replaced = self.replace_child(last_child_idx, updated_child).unwrap();
                        PushResult::Update(replaced)
                    }
                    PushResult::Full => {
                        if self.length() == COUNT {
                            // 부모 노드도 꽉 찼다면 더 이상 자식을 추가할 수 없음
                            PushResult::Full
                        } else {
                            // 자식 레벨(self.level - 1)에 맞는 singleton 생성
                            let new_child = Self::singleton(self.level() - 1, value);
                            PushResult::Update(self.append_child(new_child).unwrap())
                        }
                    }
                }
            }
        }
    }

    fn pop(&self) -> PopResult<T, COUNT> {
        match self {
            Self::Leaf(leaf) => {
                if leaf.values.length() == 1 {
                    return PopResult::Empty;
                }

                let mut node = self.clone();

                node.leaf_mut().values.pop();

                PopResult::Update(Arc::new(node))
            }

            Self::Branch(branch) => {
                let last = branch.children.length() - 1;

                let child = unsafe { branch.children.get_unchecked(last) };

                match child.pop() {
                    PopResult::Update(new_child) => {
                        let mut node = self.clone();

                        node.branch_mut().children.replace(last, new_child);

                        PopResult::Update(Arc::new(node))
                    }

                    PopResult::Empty => {
                        let mut node = self.clone();

                        node.branch_mut().children.pop();

                        if node.branch().children.length() == 0 {
                            PopResult::Empty
                        } else {
                            PopResult::Update(Arc::new(node))
                        }
                    }
                }
            }
        }
    }

    fn truncate(&self, len: usize) -> TruncateResult<T, COUNT> {
        match self {
            Self::Leaf(leaf) => {
                if len == 0 {
                    return TruncateResult::Empty;
                }

                let mut node = self.clone();

                node.leaf_mut().values.truncate(len);

                TruncateResult::Node(Arc::new(node))
            }

            Self::Branch(branch) => {
                let span = self.child_span();

                let child_index = len / span;
                let child_len = len % span;

                let mut node = self.clone();

                if child_len == 0 {
                    node.branch_mut().children.truncate(child_index);
                } else {
                    let child = unsafe { branch.children.get_unchecked(child_index) };

                    match child.truncate(child_len) {
                        TruncateResult::Empty => {
                            node.branch_mut().children.truncate(child_index);
                        }

                        TruncateResult::Node(new_child) => {
                            node.branch_mut().children.replace(child_index, new_child);

                            node.branch_mut().children.truncate(child_index + 1);
                        }
                    }
                }

                if node.branch().children.length() == 0 {
                    TruncateResult::Empty
                } else {
                    TruncateResult::Node(Arc::new(node))
                }
            }
        }
    }
}

impl<'a, T: ?Sized, const CAPACITY: usize> IntoIterator for &'a InnerPersistentVector<T, CAPACITY> {
    type Item = Arc<T>;
    type IntoIter = Iter<T, CAPACITY>;

    fn into_iter(self) -> Self::IntoIter {
        Iter::new(self.root.clone(), self.size)
    }
}

impl<'a, T: ?Sized, const CAPACITY: usize> IntoIterator for InnerPersistentVector<T, CAPACITY> {
    type Item = Arc<T>;
    type IntoIter = Iter<T, CAPACITY>;

    fn into_iter(self) -> Self::IntoIter {
        Iter::new(self.root, self.size)
    }
}

impl<T, const CAPACITY: usize> FromIterator<T> for InnerPersistentVector<T, CAPACITY> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut result = Self::new();

        for value in iter {
            result = result.push(Arc::new(value));
        }

        result
    }
}

impl<T: PartialEq + ?Sized, const CAPACITY: usize> PartialEq for InnerPersistentVector<T, CAPACITY> {
    fn eq(&self, other: &Self) -> bool {
        if self.size != other.size {
            return false;
        }

        for index in 0..self.size {
            if self.get(index) != other.get(index) {
                return false;
            }
        }

        true
    }
}

impl<T: Eq + ?Sized, const CAPACITY: usize> Eq for InnerPersistentVector<T, CAPACITY> {}

impl<T: Debug + ?Sized, const CAPACITY: usize> Debug for InnerPersistentVector<T, CAPACITY> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut debug_list = f.debug_list();

        for value in self.iter() {
            debug_list.entry(&value);
        }

        debug_list.finish()
    }
}

impl<T: ?Sized, const CAPACITY: usize> Clone for InnerPersistentVector<T, CAPACITY> {
    fn clone(&self) -> Self {
        Self {
            root: self.root.clone(),
            size: self.size,
            height: self.height,
        }
    }
}

impl<T: ?Sized, const COUNT: usize> Clone for Node<T, COUNT> {
    fn clone(&self) -> Self {
        match self {
            Node::Branch(branch) => Self::Branch(branch.clone()),
            Node::Leaf(leaf) => Self::Leaf(leaf.clone()),
        }
    }
}

enum TruncateResult<T: ?Sized, const COUNT: usize> {
    Node(Arc<Node<T, COUNT>>),
    Empty,
}

enum PopResult<T: ?Sized, const COUNT: usize> {
    Update(Arc<Node<T, COUNT>>),
    Empty,
}

enum PushResult<T: ?Sized, const COUNT: usize> {
    Update(Arc<Node<T, COUNT>>),
    Full,
}

impl<T: ?Sized, const COUNT: usize> PushResult<T, COUNT> {
    fn is_full(&self) -> bool {
        matches!(self, Self::Full)
    }

    fn is_update(&self) -> bool {
        !self.is_full()
    }
}

enum ReplaceChildResult<T: ?Sized, const COUNT: usize> {
    InvalidIndex,
    NotBranch,
    InvalidLevel,
    ReplacedNode(Arc<Node<T, COUNT>>),
}

impl<T: ?Sized, const COUNT: usize> ReplaceChildResult<T, COUNT> {
    fn unwrap(self) -> Arc<Node<T, COUNT>> {
        let ReplaceChildResult::ReplacedNode(node) = self else {
            panic!("called `ReplaceChildResult::unwrap()` on a not `ReplacedNode` value")
        };
        node
    }
}

enum AppendChildResult<T: ?Sized, const COUNT: usize> {
    Full,
    NotBranch,
    InvalidLevel,
    AppendNode(Arc<Node<T, COUNT>>),
}

impl<T: ?Sized, const COUNT: usize> AppendChildResult<T, COUNT> {
    fn unwrap(self) -> Arc<Node<T, COUNT>> {
        let AppendChildResult::AppendNode(node) = self else {
            panic!("called `AppendChildResult::unwrap()` on a not `AppendNode` value")
        };
        node
    }
}

#[inline]
fn uninit_array<T, const N: usize>() -> [MaybeUninit<T>; N] {
    unsafe { MaybeUninit::uninit().assume_init() }
}
