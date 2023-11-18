use std::collections::{btree_set, BTreeSet};
use std::fmt::Debug;
use std::ops::AddAssign;
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

        eprintln!("split {:p} creating {:?}: \n{:?}\n{:?}", self, gt_node, self.values, unsafe {
            &(*gt_node).values
        });

        gt_node
    }

    // pub fn get_separator(
    //     &mut self,
    //     other: Option<*mut Node<K, V>>,
    //     og: *mut Node<K, V>, // if self == other, use og in place of self
    // ) -> Option<Slot<K, V>> {
    //     // other.map(|raw_gt_node| {
    //     //     let me = (|node: *mut Node<K, V>| {
    //     //         let me = if node == raw_gt_node { og } else { node };
    //     //         unsafe { &mut (*me) }
    //     //     })(self);

    //     //     let rk = me.last_k().expect("self should have a last slot");
    //     //     let mut rs = Slot::new_internal(rk, me);

    //     //     let gt_node = unsafe { &mut (*raw_gt_node) };
    //     //     let gtk = gt_node.last_k().expect("gt should have a last slot");
    //     //     let mut ns = Slot::new_internal(gtk, raw_gt_node);

    //     //     if me.is_leaf() {
    //     //         rs.0.increment();
    //     //         ns.0.increment();
    //     //     }

    //     //     (rs, ns)
    //     // })

    //     other.map(|raw_node| {

    //         let node = unsafe { &*raw_node };
    //         let s = node.values.first().map(|s| s.0).unwrap();

    //         todo!()
    //     })
    // }

    pub fn get_separator(
        ptr: *mut Node<K, V>,
        other: Option<*mut Node<K, V>>,
    ) -> Option<(Slot<K, V>, *mut Node<K, V>)> {
        other.map(|optr| {
            let other = unsafe { &*optr };
            let k = other.values.first().map(|s| s.0).unwrap();
            let s = Slot::new_internal(k, ptr);

            (s, optr)
        })
    }

    /// Returns `None` if self is a leaf.
    pub fn find_child(&self, value: Slot<K, V>) -> Option<*mut Node<K, V>> {
        if self.is_leaf() {
            return None;
        }

        let n = self.values.iter().find(|n| value < **n)?;
        Some(get_right!(n))
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub fn almost_full(&self) -> bool {
        self.values.len() >= self.max / 2
    }

    pub fn insert(&mut self, slot: Slot<K, V>) {
        self.values.insert(slot);
    }

    pub fn replace(&mut self, slot: Slot<K, V>) {
        self.values.replace(slot);
    }

    pub fn delete(&mut self, slot: &Slot<K, V>) -> bool {
        self.values.remove(slot)
    }

    pub fn get(&self, slot: &Slot<K, V>) -> Option<&Slot<K, V>> {
        self.values.get(slot)
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
