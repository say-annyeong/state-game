use std::{iter::FusedIterator, sync::Arc};

use super::Node;

pub struct Iter<T: ?Sized, const COUNT: usize> {
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
struct Cursor<T: ?Sized, const COUNT: usize> {
    stack: Vec<Frame<T, COUNT>>,
}

struct Frame<T: ?Sized, const COUNT: usize> {
    node: Arc<Node<T, COUNT>>,
    index: usize,
}

impl<T: ?Sized, const COUNT: usize> Iter<T, COUNT> {
    pub(super) fn new(root: Option<Arc<Node<T, COUNT>>>, size: usize) -> Self {
        let front = Cursor::new(root.clone(), false);
        let back = Cursor::new(root, true);

        Self {
            front,
            back,
            remaining: size,
        }
    }
}

impl<T: ?Sized, const COUNT: usize> Cursor<T, COUNT> {
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

impl<T: ?Sized, const COUNT: usize> Iterator for Iter<T, COUNT> {
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

impl<T: ?Sized, const COUNT: usize> DoubleEndedIterator for Iter<T, COUNT> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }

        let value = self.back.next_back()?;

        self.remaining -= 1;

        Some(value)
    }
}

impl<T: ?Sized, const COUNT: usize> ExactSizeIterator for Iter<T, COUNT> {
    fn len(&self) -> usize {
        self.remaining
    }
}

impl<T: ?Sized, const COUNT: usize> FusedIterator for Iter<T, COUNT> {}

impl<T: ?Sized, const COUNT: usize> Clone for Iter<T, COUNT> {
    fn clone(&self) -> Self {
        Self {
            front: self.front.clone(),
            back: self.back.clone(),
            remaining: self.remaining,
        }
    }
}

impl<T: ?Sized, const COUNT: usize> Clone for Cursor<T, COUNT> {
    fn clone(&self) -> Self {
        Self {
            stack: self.stack.clone(),
        }
    }
}

impl<T: ?Sized, const COUNT: usize> Clone for Frame<T, COUNT> {
    fn clone(&self) -> Self {
        Self {
            node: self.node.clone(),
            index: self.index,
        }
    }
}
