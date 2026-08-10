use std::fmt::{Debug, Formatter};
use std::iter::FusedIterator;
use std::mem::MaybeUninit;
use std::sync::Arc;

const NODE_CAPACITY: usize = 32; // todo: 4 -> 32

/// Invariants:
///
/// - `size == 0` if and only if `root.is_none()` and `tail.is_none()`.
/// - `size > 0` if and only if either `root.is_some()` or `tail.is_some()`.
/// - `height == 0` if and only if `root.is_none()`.
/// - If `root.is_some()`, then `height == root.level()`.
/// - Every reachable node satisfies `Node`'s invariants.
/// - Every leaf node inside `root` is at the same depth (`height`) from the root.
/// - `tail` contains the newest elements that do not belong to the tree.
///
/// - Elements are stored in index order without gaps:
///   - All elements in `root` precede all elements in `tail`.
///   - `tail` is empty only when `tail.is_none()`.
///
/// - The total number of stored elements is:
///   - the sum of all values stored in reachable leaf nodes of `root`,
///   - plus the number of values stored in `tail`.
///
/// - `tail`, when present, is always a valid `LeafNode`.
/// - `tail` length is in the range `1..=CAPACITY`.
/// - If `tail` is full, pushing another element must flush it into `root`.
pub struct PersistentVector<T> {
    root: Option<Arc<Node<T, NODE_CAPACITY>>>,
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
enum Node<T, const COUNT: usize> {
    Branch(BranchNode<T, COUNT>),
    Leaf(LeafNode<T, COUNT>),
}

impl<T> PersistentVector<T> {
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
        NODE_CAPACITY.pow(self.height as u32)
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
            return None
        }

        self.get(self.size - 1)
    }

    pub fn get(&self, index: usize) -> Option<Arc<T>> {
        if index >= self.size {
            return None;
        }

        self.root.as_ref()?.get(index)
    }

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

    pub fn iter(&self) -> Iter<T, NODE_CAPACITY> {
        Iter::new(self.root.clone(), self.size)
    }

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

impl<T, const COUNT: usize> Node<T, COUNT> {
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

        node.branch_mut().children.data[index].write(child);

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

impl<'a, T> IntoIterator for &'a PersistentVector<T> {
    type Item = Arc<T>;
    type IntoIter = Iter<T, NODE_CAPACITY>;

    fn into_iter(self) -> Self::IntoIter {
        Iter::new(self.root.clone(), self.size)
    }
}

impl<T> IntoIterator for PersistentVector<T> {
    type Item = Arc<T>;
    type IntoIter = Iter<T, NODE_CAPACITY>;

    fn into_iter(self) -> Self::IntoIter {
        Iter::new(self.root, self.size)
    }
}

impl<T> FromIterator<T> for PersistentVector<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut result = Self::new();

        for value in iter {
            result = result.push(Arc::new(value));
        }

        result
    }
}

impl<T: PartialEq> PartialEq for PersistentVector<T> {
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

impl<T: Eq> Eq for PersistentVector<T> {}

impl<T: Debug> Debug for PersistentVector<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut debug_list = f.debug_list();

        for value in self.iter() {
            debug_list.entry(&value);
        }

        debug_list.finish()
    }
}

impl<T> Clone for PersistentVector<T> {
    fn clone(&self) -> Self {
        Self {
            root: self.root.clone(),
            size: self.size,
            height: self.height,
        }
    }
}

impl<T, const COUNT: usize> Clone for Node<T, COUNT> {
    fn clone(&self) -> Self {
        match self {
            Node::Branch(branch) => Self::Branch(branch.clone()),
            Node::Leaf(leaf) => Self::Leaf(leaf.clone()),
        }
    }
}

/// Invariants:
///
/// - 0 <= length <= COUNT.
/// - All slots in range `0..length` are initialized.
/// - Slots in range `length..COUNT` may be initialized or uninitialized.
/// - Access to slots outside `0..length` is not allowed unless through
///   initialization APIs.
struct NodeArray<T, const COUNT: usize> {
    data: [MaybeUninit<Arc<T>>; COUNT],
    length: usize,
}

impl<T, const COUNT: usize> NodeArray<T, COUNT> {
    #[inline(always)]
    fn new() -> Self {
        Self {
            data: unsafe { MaybeUninit::uninit().assume_init() },
            length: 0,
        }
    }

    #[inline(always)]
    fn length(&self) -> usize {
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
    unsafe fn first_unchecked(&self) -> &Arc<T> {
        unsafe { self.data[0].assume_init_ref() }
    }

    #[inline(always)]
    fn first(&self) -> Option<&Arc<T>> {
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
    unsafe fn first_mut_unchecked(&mut self) -> &mut Arc<T> {
        unsafe { self.data[0].assume_init_mut() }
    }

    fn first_mut(&mut self) -> Option<&mut Arc<T>> {
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
    unsafe fn last_unchecked(&self) -> &Arc<T> {
        unsafe { self.data[self.length - 1].assume_init_ref() }
    }

    #[inline(always)]
    fn last(&self) -> Option<&Arc<T>> {
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
    unsafe fn last_mut_unchecked(&mut self) -> &mut Arc<T> {
        unsafe { self.data[self.length - 1].assume_init_mut() }
    }

    #[inline(always)]
    fn last_mut(&mut self) -> Option<&mut Arc<T>> {
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
    unsafe fn get_unchecked(&self, index: usize) -> &Arc<T> {
        unsafe { self.data[index].assume_init_ref() }
    }

    #[inline(always)]
    fn get(&self, index: usize) -> Option<&Arc<T>> {
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
    unsafe fn get_mut_unchecked(&mut self, index: usize) -> &mut Arc<T> {
        unsafe { self.data[index].assume_init_mut() }
    }

    #[inline(always)]
    fn get_mut(&mut self, index: usize) -> Option<&mut Arc<T>> {
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
    unsafe fn replace_unchecked(&mut self, index: usize, value: Arc<T>) -> Arc<T> {
        let old = unsafe { self.data[index].assume_init_read() };

        self.data[index].write(value);

        old
    }

    #[inline(always)]
    fn replace(&mut self, index: usize, value: Arc<T>) -> Option<Arc<T>> {
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
    unsafe fn pop_unchecked(&mut self) -> Arc<T> {
        self.length -= 1;

        unsafe { self.data[self.length].assume_init_read() }
    }

    #[inline(always)]
    fn pop(&mut self) -> Option<Arc<T>> {
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
    unsafe fn push_unchecked(&mut self, value: Arc<T>) {
        self.data[self.length].write(value);
        self.length += 1;
    }

    #[inline(always)]
    fn push(&mut self, value: Arc<T>) -> bool {
        if self.length == COUNT {
            return false;
        }

        unsafe { self.push_unchecked(value) };
        true
    }

    #[inline(always)]
    fn as_slice(&self) -> &[Arc<T>] {
        unsafe { std::slice::from_raw_parts(self.data.as_ptr() as *const Arc<T>, self.length) }
    }

    #[inline(always)]
    fn as_mut_slice(&mut self) -> &mut [Arc<T>] {
        unsafe {
            std::slice::from_raw_parts_mut(self.data.as_mut_ptr() as *mut Arc<T>, self.length)
        }
    }

    #[inline(always)]
    fn validate(&self) -> Result<(), String> {
        if self.length > COUNT {
            return Err(format!(
                "length must be in the range 0..={COUNT}, got {}",
                self.length
            ));
        }

        Ok(())
    }

    #[inline(always)]
    fn truncate(&mut self, len: usize) {
        while self.length > len {
            self.length -= 1;

            unsafe {
                self.data[self.length].assume_init_drop();
            }
        }
    }
}

impl<T, const COUNT: usize> Clone for NodeArray<T, COUNT> {
    fn clone(&self) -> Self {
        let mut result = Self::new();

        for index in 0..self.length {
            result.data[index].write(
                unsafe { self.data[index].assume_init_ref().clone() }
            );
        }

        result.length = self.length;

        result
    }
}

impl<T, const COUNT: usize> Drop for NodeArray<T, COUNT> {
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
struct BranchNode<T, const COUNT: usize> {
    children: NodeArray<Node<T, COUNT>, COUNT>,
    level: usize,
}

/// LeafNode invariants:
///
/// - 0 < values.length() <= COUNT.
/// - All stored values are initialized.
/// - No gaps exist.
struct LeafNode<T, const COUNT: usize> {
    values: NodeArray<T, COUNT>,
}

impl<T, const COUNT: usize> BranchNode<T, COUNT> {
    #[inline(always)]
    fn new(children: NodeArray<Node<T, COUNT>, COUNT>, level: usize) -> Option<Self> {
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
    unsafe fn new_unchecked(children: NodeArray<Node<T, COUNT>, COUNT>, level: usize) -> Self {
        Self::new_raw(children, level)
    }

    #[inline(always)]
    fn new_raw(children: NodeArray<Node<T, COUNT>, COUNT>, level: usize) -> Self {
        Self { children, level }
    }

    fn validate(&self) -> Result<(), String> {
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
    fn push_child(&mut self, child: Arc<Node<T, COUNT>>) {
        self.children.push(child);
    }

    #[inline(always)]
    fn get(&self, index: usize) -> Option<&Arc<Node<T, { COUNT }>>> {
        self.children.get(index)
    }

    #[inline(always)]
    fn child_span(&self) -> usize {
        COUNT.pow((self.level) as u32)
    }

    #[inline(always)]
    fn span(&self) -> usize {
        self.child_span() * self.children.length
    }

    #[inline(always)]
    fn max_capacity(&self) -> usize {
        self.child_span() * COUNT
    }
}

impl<T, const COUNT: usize> LeafNode<T, COUNT> {
    const LEVEL: usize = 0;
    const CHILD_SPAN: usize = 1;

    #[inline(always)]
    fn new(values: NodeArray<T, COUNT>) -> Option<Self> {
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
    unsafe fn new_unchecked(values: NodeArray<T, COUNT>) -> Self {
       Self::new_raw(values)
    }

    #[inline(always)]
    fn new_raw(values: NodeArray<T, COUNT>) -> Self {
        Self { values }
    }

    #[inline(always)]
    fn validate(&self) -> Result<(), String> {
        self.values.validate()?;

        if self.values.length() == 0 {
            return Err("leaf node must contain at least one value".to_string());
        }

        Ok(())
    }

    #[inline(always)]
    fn get(&self, index: usize) -> Option<&Arc<T>> {
        self.values.get(index)
    }

    #[inline(always)]
    fn span(&self) -> usize {
        self.values.length
    }
}

impl<T, const CAPACITY: usize> Clone for BranchNode<T, CAPACITY> {
    fn clone(&self) -> Self {
        Self {
            children: self.children.clone(),
            level: self.level,
        }
    }
}

impl<T, const CAPACITY: usize> Clone for LeafNode<T, CAPACITY> {
    fn clone(&self) -> Self {
        Self {
            values: self.values.clone(),
        }
    }
}

pub struct Iter<T, const COUNT: usize> {
    front: Cursor<T, COUNT>,
    back: Cursor<T, COUNT>,
    remaining: usize,
}

/// Cursor invariants:
///
/// - Every frame points to a reachable node.
/// - For leaf nodes, `index` is in the range `0..=length`.
/// - For branch nodes, `index` is in the range `0..=length`.
/// - Frames form a valid path from the root to the current node.
/// - The top frame represents the next node to visit.
struct Cursor<T, const COUNT: usize> {
    stack: Vec<Frame<T, COUNT>>,
}

struct Frame<T, const COUNT: usize> {
    node: Arc<Node<T, COUNT>>,
    index: usize,
}

impl<T, const COUNT: usize> Iter<T, COUNT> {
    fn new(root: Option<Arc<Node<T, COUNT>>>, size: usize) -> Self {
        let front = Cursor::new(root.clone(), false);
        let back = Cursor::new(root, true);

        Self {
            front,
            back,
            remaining: size,
        }
    }
}

impl<T, const COUNT: usize> Cursor<T, COUNT> {
    fn new(root: Option<Arc<Node<T, COUNT>>>, reverse: bool) -> Self {
        let mut stack = Vec::new();

        if let Some(root) = root {
            let index = if reverse { root.length() } else { 0 };

            stack.push(Frame { node: root, index });
        }

        Self { stack }
    }

    fn next(&mut self) -> Option<Arc<T>> {
        loop {
            let frame = self.stack.last_mut()?;

            if frame.node.is_leaf() {
                if frame.index >= frame.node.length() {
                    self.stack.pop();
                    continue;
                }

                let value = unsafe { frame.node.leaf().values.get_unchecked(frame.index).clone() };

                frame.index += 1;

                return Some(value);
            }

            if frame.index >= frame.node.length() {
                self.stack.pop();
                continue;
            }

            let child = unsafe {
                frame
                    .node
                    .branch()
                    .children
                    .get_unchecked(frame.index)
                    .clone()
            };

            frame.index += 1;

            self.stack.push(Frame {
                node: child,
                index: 0,
            });
        }
    }

    fn next_back(&mut self) -> Option<Arc<T>> {
        loop {
            let frame = self.stack.last_mut()?;

            if frame.node.is_leaf() {
                if frame.index == 0 {
                    self.stack.pop();
                    continue;
                }

                frame.index -= 1;

                let value = unsafe { frame.node.leaf().values.get_unchecked(frame.index).clone() };

                return Some(value);
            }

            if frame.index == 0 {
                self.stack.pop();
                continue;
            }

            frame.index -= 1;

            let child = unsafe {
                frame
                    .node
                    .branch()
                    .children
                    .get_unchecked(frame.index)
                    .clone()
            };

            self.stack.push(Frame {
                index: child.length(),
                node: child,
            });
        }
    }
}

impl<T, const COUNT: usize> Iterator for Iter<T, COUNT> {
    type Item = Arc<T>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }

        let value = self.front.next()?;

        self.remaining -= 1;

        Some(value)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl<T, const COUNT: usize> DoubleEndedIterator for Iter<T, COUNT> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }

        let value = self.back.next_back()?;

        self.remaining -= 1;

        Some(value)
    }
}

impl<T, const COUNT: usize> ExactSizeIterator for Iter<T, COUNT> {
    fn len(&self) -> usize {
        self.remaining
    }
}

impl<T, const COUNT: usize> FusedIterator for Iter<T, COUNT> {}

impl<T, const COUNT: usize> Clone for Iter<T, COUNT> {
    fn clone(&self) -> Self {
        Self {
            front: self.front.clone(),
            back: self.back.clone(),
            remaining: self.remaining,
        }
    }
}

impl<T, const COUNT: usize> Clone for Cursor<T, COUNT> {
    fn clone(&self) -> Self {
        Self {
            stack: self.stack.clone(),
        }
    }
}

impl<T, const COUNT: usize> Clone for Frame<T, COUNT> {
    fn clone(&self) -> Self {
        Self {
            node: self.node.clone(),
            index: self.index,
        }
    }
}

enum TruncateResult<T, const COUNT: usize> {
    Node(Arc<Node<T, COUNT>>),
    Empty,
}

enum PopResult<T, const COUNT: usize> {
    Update(Arc<Node<T, COUNT>>),
    Empty,
}

enum PushResult<T, const COUNT: usize> {
    Update(Arc<Node<T, COUNT>>),
    Full,
}

impl<T, const COUNT: usize> PushResult<T, COUNT> {
    fn is_full(&self) -> bool {
        matches!(self, Self::Full)
    }

    fn is_update(&self) -> bool {
        !self.is_full()
    }
}

enum ReplaceChildResult<T, const COUNT: usize> {
    InvalidIndex,
    NotBranch,
    InvalidLevel,
    ReplacedNode(Arc<Node<T, COUNT>>),
}

impl<T, const COUNT: usize> ReplaceChildResult<T, COUNT> {
    fn unwrap(self) -> Arc<Node<T, COUNT>> {
        let ReplaceChildResult::ReplacedNode(node) = self else {
            panic!("called `ReplaceChildResult::unwrap()` on a not `ReplacedNode` value")
        };
        node
    }
}

enum AppendChildResult<T, const COUNT: usize> {
    Full,
    NotBranch,
    InvalidLevel,
    AppendNode(Arc<Node<T, COUNT>>),
}

impl<T, const COUNT: usize> AppendChildResult<T, COUNT> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    // 1. 초기 생성 및 빈 벡터 상태 검증
    #[test]
    fn test_new_and_empty() {
        let vec: PersistentVector<i32> = PersistentVector::new();
        assert_eq!(vec.len(), 0);
        assert!(vec.is_empty());
        assert_eq!(vec.height(), 0);
        assert_eq!(vec.first(), None);
        assert_eq!(vec.last(), None);
        assert_eq!(vec.get(0), None);
    }

    // 2. Push 및 영속성(Persistence, 구조적 공유) 검증
    #[test]
    fn test_push_and_persistence() {
        let v0: PersistentVector<i32> = PersistentVector::new();
        let v1 = v0.push(Arc::new(10));
        let v2 = v1.push(Arc::new(20));
        let v3 = v2.push(Arc::new(30));

        // 이전 버전의 벡터 데이터가 전혀 변경되지 않고 유지되는지 확인 (불변성 검증)
        assert_eq!(v0.len(), 0);
        assert_eq!(v1.len(), 1);
        assert_eq!(v2.len(), 2);
        assert_eq!(v3.len(), 3);

        assert_eq!(*v1.get(0).unwrap(), 10);

        assert_eq!(*v2.get(0).unwrap(), 10);
        assert_eq!(*v2.get(1).unwrap(), 20);

        assert_eq!(*v3.get(0).unwrap(), 10);
        assert_eq!(*v3.get(1).unwrap(), 20);
        assert_eq!(*v3.get(2).unwrap(), 30);

        assert_eq!(*v3.first().unwrap(), 10);
        assert_eq!(*v3.last().unwrap(), 30);
    }

    // 3. 많은 원소 삽입 시 트리 레벨 확장(NODE_CAPACITY=4 초과) 검증
    #[test]
    fn test_large_push_and_tree_expansion() {
        let mut vec = PersistentVector::new();
        let count = 200;

        for i in 0..count {
            vec = vec.push(Arc::new(i));
            assert_eq!(vec.len(), i + 1);
            assert_eq!(*vec.get(i).unwrap(), i);
        }

        assert_eq!(vec.len(), count);
        assert!(vec.height() > 0); // 트리 높이가 상향되었는지 확인

        for i in 0..count {
            assert_eq!(*vec.get(i).unwrap(), i);
        }
    }

    // 4. 특정 인덱스 값 수정(set) 및 클로저 기반 업데이트(update) 검증
    #[test]
    fn test_set_and_update() {
        let mut vec = PersistentVector::new();
        for i in 0..20 {
            vec = vec.push(Arc::new(i * 10));
        }

        // set 검증
        let vec_set = vec.set(5, 999).unwrap();
        assert_eq!(*vec.get(5).unwrap(), 50);      // 원본은 그대로 유지 (50)
        assert_eq!(*vec_set.get(5).unwrap(), 999);  // 새 벡터만 변경 (999)
        assert_eq!(vec_set.len(), 20);

        // 범위를 벗어난 인덱스 오류 처리
        assert!(vec.set(100, 500).is_err());

        // update 검증 (값 기반 계산 업데이트)
        let vec_updated = vec.update(5, |val| val + 1).unwrap();
        assert_eq!(*vec_updated.get(5).unwrap(), 51);
        assert!(vec.update(100, |val| val + 1).is_err());
    }

    // 5. Pop 기능 및 트리 축소 검증
    #[test]
    fn test_pop() {
        let mut vec = PersistentVector::new();
        for i in 0..15 {
            vec = vec.push(Arc::new(i));
        }

        let mut current = vec;
        for i in (0..15).rev() {
            assert_eq!(current.len(), i + 1);
            assert_eq!(*current.last().unwrap(), i);
            current = current.pop().unwrap();
        }

        assert_eq!(current.len(), 0);
        assert!(current.is_empty());
        assert!(current.pop().is_none()); // 빈 벡터 pop 시 None 반환
    }

    // 6. Truncate (길이 잘라내기) 및 Clear 검증
    #[test]
    fn test_truncate_and_clear() {
        let mut vec = PersistentVector::new();
        for i in 0..30 {
            vec = vec.push(Arc::new(i));
        }

        let truncated = vec.truncate(10);
        assert_eq!(truncated.len(), 10);
        assert_eq!(vec.len(), 30); // 원본은 유지

        for i in 0..10 {
            assert_eq!(*truncated.get(i).unwrap(), i);
        }

        let cleared = vec.clear();
        assert_eq!(cleared.len(), 0);
        assert!(cleared.is_empty());
    }

    // 7. Append, Extend, Insert, Remove 기능 검증
    #[test]
    fn test_structure_modifications() {
        let mut vec1 = PersistentVector::new();
        for i in 0..5 {
            vec1 = vec1.push(Arc::new(i));
        }

        let mut vec2 = PersistentVector::new();
        for i in 5..10 {
            vec2 = vec2.push(Arc::new(i));
        }

        // append & extend
        let appended = vec1.append(&vec2);
        assert_eq!(appended.len(), 10);
        for i in 0..10 {
            assert_eq!(*appended.get(i).unwrap(), i);
        }

        // insert
        let inserted = vec1.insert(2, 99); // [0, 1, 99, 2, 3, 4]
        assert_eq!(inserted.len(), 6);
        assert_eq!(*inserted.get(2).unwrap(), 99);
        assert_eq!(*inserted.get(3).unwrap(), 2);

        // remove
        let removed = inserted.remove(2); // 다시 99 제거 -> [0, 1, 2, 3, 4]
        assert_eq!(removed.len(), 5);
        assert_eq!(*removed.get(2).unwrap(), 2);
    }

    // 8. Iter 및 DoubleEndedIterator(양방향 순회) 검증
    #[test]
    fn test_iterators() {
        let mut vec = PersistentVector::new();
        for i in 0..25 {
            vec = vec.push(Arc::new(i));
        }

        // 순방향 순회
        let forward: Vec<i32> = vec.iter().map(|x| *x).collect();
        let expected: Vec<i32> = (0..25).collect();
        assert_eq!(forward, expected);

        // 역방향 순회 (DoubleEndedIterator)
        let backward: Vec<i32> = vec.iter().rev().map(|x| *x).collect();
        let expected_rev: Vec<i32> = (0..25).rev().collect();
        assert_eq!(backward, expected_rev);

        // IntoIterator (참조 기반)
        let into_iter_vals: Vec<i32> = (&vec).into_iter().map(|x| *x).collect();
        assert_eq!(into_iter_vals, expected);

        // ExactSizeIterator 남은 개수 검증
        let mut iter = vec.iter();
        assert_eq!(iter.len(), 25);
        iter.next();
        assert_eq!(iter.len(), 24);
    }

    // 9. 정렬 관련 API 세트 전체 검증
    #[test]
    fn test_sorting_functions() {
        let nums = vec![42, 12, 88, 3, 15, 27, 99, 1];
        let mut vec = PersistentVector::new();
        for &n in &nums {
            vec = vec.push(Arc::new(n));
        }

        // 1) 기본 오름차순 정렬
        let sorted: Vec<i32> = vec.sort().iter().map(|x| *x).collect();
        assert_eq!(sorted, vec![1, 3, 12, 15, 27, 42, 88, 99]);

        // 2) Unstable 오름차순 정렬
        let sorted_unstable: Vec<i32> = vec.sort_unstable().iter().map(|x| *x).collect();
        assert_eq!(sorted_unstable, vec![1, 3, 12, 15, 27, 42, 88, 99]);

        // 3) 커스텀 비교 함수 (내림차순 정렬)
        let sorted_by: Vec<i32> = vec.sort_by(|a, b| b.cmp(a)).iter().map(|x| *x).collect();
        assert_eq!(sorted_by, vec![99, 88, 42, 27, 15, 12, 3, 1]);

        // 4) 키 추출 함수 정렬 (10의 자리 기준 정렬)
        let sorted_key: Vec<i32> = vec.sort_by_key(|&x| x % 10).iter().map(|x| *x).collect();
        assert_eq!(*sorted_key.first().unwrap(), 1); // 1 % 10 = 1

        // 5) 캐시된 키 정렬 (문자열 변환 기준 정렬)
        let sorted_cached: Vec<i32> = vec.sort_by_cached_key(|&x| x.to_string()).iter().map(|x| *x).collect();
        assert_eq!(sorted_cached.len(), nums.len());
    }
}

#[cfg(test)]
mod stress_tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn test_random_operations_against_vec() {
        let mut pv = PersistentVector::new();
        let mut vec = Vec::new();

        let mut seed = 0x12345678u64;

        fn rand(seed: &mut u64) -> usize {
            *seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);

            (*seed >> 32) as usize
        }

        for _ in 0..10000 {
            match rand(&mut seed) % 5 {
                // push
                0 => {
                    let value = rand(&mut seed) as i32;

                    pv = pv.push(Arc::new(value));
                    vec.push(value);
                }

                // set
                1 => {
                    if !vec.is_empty() {
                        let index = rand(&mut seed) % vec.len();
                        let value = rand(&mut seed) as i32;

                        pv = pv.set(index, value).unwrap();
                        vec[index] = value;
                    }
                }

                // pop
                2 => {
                    if !vec.is_empty() {
                        pv = pv.pop().unwrap();
                        vec.pop();
                    }
                }

                // get
                3 => {
                    if !vec.is_empty() {
                        let index = rand(&mut seed) % vec.len();

                        assert_eq!(
                            *pv.get(index).unwrap(),
                            vec[index]
                        );
                    }
                }

                // iterator check
                4 => {
                    let collected: Vec<_> =
                        pv.iter().map(|x| *x).collect();

                    assert_eq!(collected, vec);
                }

                _ => unreachable!(),
            }

            assert_eq!(pv.len(), vec.len());

            for i in 0..vec.len() {
                assert_eq!(
                    *pv.get(i).unwrap(),
                    vec[i]
                );
            }
        }
    }


    #[test]
    fn test_large_tree_integrity() {
        let mut pv = PersistentVector::new();

        let count = 100_000;

        for i in 0..count {
            pv = pv.push(Arc::new(i));
        }

        assert_eq!(pv.len(), count);

        for i in 0..count {
            assert_eq!(
                *pv.get(i).unwrap(),
                i
            );
        }

        let collected: Vec<_> =
            pv.iter().map(|x| *x).collect();

        assert_eq!(
            collected.len(),
            count
        );

        assert_eq!(
            collected[0],
            0
        );

        assert_eq!(
            collected[count - 1],
            count - 1
        );
    }


    #[test]
    fn test_persistence_after_many_versions() {
        let mut versions = Vec::new();

        let mut current = PersistentVector::new();

        for i in 0..1000 {
            current = current.push(Arc::new(i));
            versions.push(current.clone());
        }

        for (index, version) in versions.iter().enumerate() {
            assert_eq!(
                version.len(),
                index + 1
            );

            assert_eq!(
                *version.last().unwrap(),
                index
            );
        }
    }


    #[test]
    fn test_set_does_not_mutate_previous_version() {
        let mut original = PersistentVector::new();

        for i in 0..100 {
            original = original.push(Arc::new(i));
        }

        let modified =
            original.set(50, 999).unwrap();

        assert_eq!(
            *original.get(50).unwrap(),
            50
        );

        assert_eq!(
            *modified.get(50).unwrap(),
            999
        );

        for i in 0..100 {
            if i != 50 {
                assert_eq!(
                    *original.get(i).unwrap(),
                    *modified.get(i).unwrap()
                );
            }
        }
    }


    #[test]
    fn test_iterator_exact_size_and_double_end() {
        let mut pv = PersistentVector::new();

        for i in 0..100 {
            pv = pv.push(Arc::new(i));
        }

        let mut iter = pv.iter();

        assert_eq!(iter.len(), 100);

        assert_eq!(
            *iter.next().unwrap(),
            0
        );

        assert_eq!(
            *iter.next_back().unwrap(),
            99
        );

        assert_eq!(
            iter.len(),
            98
        );

        let rest: Vec<_> =
            iter.map(|x| *x).collect();

        assert_eq!(
            rest.len(),
            98
        );
    }


    #[test]
    fn test_sort_preserves_original() {
        let mut pv = PersistentVector::new();

        for i in [5, 1, 9, 3, 7] {
            pv = pv.push(Arc::new(i));
        }

        let sorted = pv.sort();

        assert_eq!(
            pv.iter().map(|x| *x).collect::<Vec<_>>(),
            vec![5,1,9,3,7]
        );

        assert_eq!(
            sorted.iter().map(|x| *x).collect::<Vec<_>>(),
            vec![1,3,5,7,9]
        );
    }
}
