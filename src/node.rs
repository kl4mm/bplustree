use std::collections::{btree_set, BTreeSet};
use std::fmt::Debug;
use std::ptr;

use crate::btree::Increment;
use crate::get_right;
use crate::slot::{Either, Slot};

#[derive(PartialEq, Debug)]
pub enum NodeType {
    Internal,
    Leaf,
}

#[derive(Debug)]
pub struct Node<K, V> {
    pub t: NodeType,
    pub values: BTreeSet<Slot<K, V>>,
    pub next: *mut Node<K, V>,
    pub max: usize,
    pub is_root: bool,
}

impl<K, V> Node<K, V>
where
    K: Copy + Debug + Ord + Increment,
    V: Copy + Debug + Eq,
{
    pub fn new_leaf(max: usize) -> Self {
        Self {
            t: NodeType::Leaf,
            values: BTreeSet::new(),
            next: ptr::null_mut(),
            max,
            is_root: false,
        }
    }

    pub fn new_internal(max: usize) -> Self {
        Self {
            t: NodeType::Internal,
            values: BTreeSet::new(),
            next: ptr::null_mut(),
            max,
            is_root: false,
        }
    }

    /// Returns greater half, new key for it and new key for replace
    pub fn split(&mut self) -> *mut Node<K, V> {
        let len = self.values.len();
        let mid = *self
            .values
            .iter()
            .nth(len / 2)
            .expect("there should be a mid slot");

        let mut gt_node = match self.t {
            NodeType::Internal => Node::new_internal(self.max),
            NodeType::Leaf => Node::new_leaf(self.max),
        };
        gt_node.values = self.values.split_off(&mid);

        let gt_node = Box::into_raw(Box::new(gt_node));
        if self.is_leaf() {
            unsafe { (*gt_node).next = self.next };
            self.next = gt_node;
        }

        gt_node
    }

    pub fn get_separator(
        ptr: *mut Node<K, V>,
        other: Option<*mut Node<K, V>>,
    ) -> Option<(Slot<K, V>, *mut Node<K, V>)> {
        other.map(|optr| {
            let me = unsafe { &*ptr };

            // Using last values for separators:
            let last = me.values.last().unwrap();
            let mut k = last.0;
            if me.is_leaf() {
                k.increment();
            }

            let s = Slot::new_internal(k, ptr);

            (s, optr)
        })
    }

    pub fn get_separators(
        ptr: *mut Node<K, V>,
        other: Option<*mut Node<K, V>>,
    ) -> Option<(Slot<K, V>, Slot<K, V>)> {
        other.map(|optr| {
            // Using last values for separators

            let me = unsafe { &*ptr };
            let ls = me.values.last().unwrap();
            let k = if me.is_leaf() { ls.0.next() } else { ls.0 };
            let s = Slot::new_internal(k, ptr);

            let o = unsafe { &*optr };
            let ls = o.values.last().unwrap();
            let k = if o.is_leaf() { ls.0.next() } else { ls.0 };
            let os = Slot::new_internal(k, optr);

            (s, os)
        })
    }

    pub fn set_last(node: &mut Node<K, V>, optr: *mut Node<K, V>) {
        let o = unsafe { &*optr };
        let ls = o.values.last().unwrap();
        let k = if o.is_leaf() { ls.0.next() } else { ls.0 };
        let s = Slot::new_internal(k, optr);
        match node.values.replace(s) {
            Some(s) => eprintln!("SLOT DISAPPEARING: {:?}", s),
            None => {}
        }
    }

    /// Returns `None` if self is a leaf.
    pub fn find_child(&self, value: Slot<K, V>) -> Option<*mut Node<K, V>> {
        if self.is_leaf() {
            return None;
        }

        let n = self.values.iter().find(|n| value < **n)?;
        Some(get_right!(n))
    }

    pub fn almost_full(&self) -> bool {
        self.values.len() >= self.max / 2
    }

    pub fn first(&self) -> Option<&Slot<K, V>> {
        self.values.first()
    }

    pub fn first_k(&self) -> Option<K> {
        self.values.first().map(|s| s.0)
    }

    pub fn first_v(&self) -> Option<Either<V, *mut Node<K, V>>> {
        self.values.first().map(|s| s.1)
    }

    pub fn last_k(&self) -> Option<K> {
        self.values.last().map(|s| s.0)
    }

    pub fn last_v(&self) -> Option<Either<V, *mut Node<K, V>>> {
        self.values.last().map(|s| s.1)
    }

    pub fn is_leaf(&self) -> bool {
        self.t == NodeType::Leaf
    }

    pub fn iter(&self) -> btree_set::Iter<Slot<K, V>> {
        self.values.iter()
    }

    #[cfg(test)]
    pub fn print(raw_node: *mut Node<K, V>)
    where
        K: std::fmt::Debug,
        V: std::fmt::Debug,
    {
        if raw_node.is_null() {
            return;
        }

        let node = unsafe { &(*raw_node) };
        match node.t {
            NodeType::Internal => {
                println!("Internal Node: {:?}", raw_node);
                println!("Contents: {:?}", node.values);
                println!("Is root: {:?}", node.is_root);
                println!("Next (should be null): {:?}", node.next);
                println!();

                for slot in &node.values {
                    match slot.1 {
                        Either::Left(_) => unreachable!(),
                        Either::Right(ptr) => Self::print(ptr),
                    }
                }
            }
            NodeType::Leaf => {
                println!("Leaf Node {:?}", raw_node);
                println!("Next: {:?}", node.next);
                println!("Contents: {:?}", node.values);
                println!();
            }
        }
    }
}
