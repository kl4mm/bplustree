use std::{collections::BTreeSet, ptr};

use rand::seq::SliceRandom;
use rand::thread_rng;

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
enum Either<A, B> {
    Left(A),
    Right(B),
}

macro_rules! get_left {
    ( $slot:ident ) => {{
        match $slot.1 {
            Either::Left(l) => l,
            Either::Right(_) => unreachable!(),
        }
    }};
}

macro_rules! get_right {
    ( $slot:ident ) => {{
        match $slot.1 {
            Either::Left(_) => unreachable!(),
            Either::Right(r) => r,
        }
    }};
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub struct Slot<A, B>(A, Either<B, *mut Node<A, B>>);

impl<A, B> PartialOrd for Slot<A, B>
where
    A: Ord,
    B: PartialEq, // Not sure why PartialEq is required
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.0.cmp(&other.0))
    }
}

impl<A, B> Ord for Slot<A, B>
where
    A: Ord,
    B: Eq,
{
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

impl<A, B> Slot<A, B> {
    pub fn new_leaf(a: A, b: B) -> Self {
        Self(a, Either::Left(b))
    }

    pub fn new_internal(a: A, node: *mut Node<A, B>) -> Self {
        Self(a, Either::Right(node))
    }

    pub fn is_leaf(&self) -> bool {
        match self.1 {
            Either::Left(_) => true,
            Either::Right(_) => false,
        }
    }
}

#[derive(PartialEq, Debug)]
pub enum NodeType {
    Internal,
    Leaf,
}

#[derive(Debug)]
pub struct Node<K, V> {
    t: NodeType,
    values: BTreeSet<Slot<K, V>>,
    next: *mut Node<K, V>,
    max: usize,
    pub is_root: bool,
}

impl<K, V> Node<K, V>
where
    K: Copy + Debug + Ord + Add<u8, Output = K>,
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

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub fn almost_full(&self) -> bool {
        self.values.len() >= self.max / 2
    }

    // if max = 8
    // node has values (0, 1, 3, 4)
    // inserting 2 creates (0, 1, 2, 3, 4)
    // mid is 2 but ends up on the self node (0, 1, 2)
    // last() returns 1 since it isn't inserted until later
    // resulting seperator is 2
    // get(2) returns None
    // solution: call last() after insert

    /// Returns greater half, new key for it and new key for replace
    pub fn split(&mut self) -> (*mut Node<K, V>, K) {
        let len = self.values.len();
        let mid = *self
            .values
            .iter()
            .nth(len / 2)
            .expect("there should be a mid slot");

        let gt = self.values.split_off(&mid);
        let gt_k = gt
            .last()
            .map(|s| s.0)
            .expect("gt should have a last element");

        let mut node = match self.t {
            NodeType::Internal => Node::new_internal(self.max),
            NodeType::Leaf => Node::new_leaf(self.max),
        };
        node.values = gt;

        // if self is a leaf node and has a next node then splitting it should set the new next
        // nodes next node to selfs next node.
        if self.t == NodeType::Leaf && !self.next.is_null() {
            node.next = self.next;
        }

        let node = Box::into_raw(Box::new(node));
        if self.t == NodeType::Leaf {
            if !self.next.is_null() {
                unsafe { (*node).next = self.next };
            }

            self.next = node;
        }

        (node, gt_k)
    }

    // TODO: check node type here? do not + 1 if internal
    pub fn get_separators(
        &mut self,
        other: Option<(*mut Node<K, V>, K)>,
    ) -> Option<(Slot<K, V>, Slot<K, V>)> {
        other.map(|(gt_node, gtk)| {
            let rk = self
                .values
                .last()
                .map(|s| s.0)
                .expect("there should be a last node");

            let rs = Slot::new_internal(rk + 1, self);
            let ns = Slot::new_internal(gtk + 1, gt_node);

            (rs, ns)
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

    pub fn is_leaf(&self) -> bool {
        self.t == NodeType::Leaf
    }
}

pub struct BTree<K, V> {
    root: *mut Node<K, V>,
    max: usize,
}

use std::fmt::Debug;
use std::ops::{Add, AddAssign};
impl<K, V> BTree<K, V>
where
    K: Clone + Copy + Debug + Add<u8, Output = K> + AddAssign<u8> + Ord + Copy,
    V: Clone + Copy + Debug + Eq,
{
    pub fn new(max: usize) -> Self {
        Self {
            root: ptr::null_mut(),
            max,
        }
    }

    pub fn insert(&mut self, entry: Slot<K, V>) {
        assert!(entry.is_leaf());

        if self.root.is_null() {
            let mut root = Node::new_internal(self.max);
            root.is_root = true;
            self.root = Box::into_raw(Box::new(root));
        }

        if let Some((old_root_slot, split_slot)) = BTree::_insert(self.root, entry) {
            assert!(get_right!(old_root_slot) == self.root);

            let root = unsafe { &mut (*self.root) };
            root.is_root = false;

            let mut new_root = Node::new_internal(self.max);
            new_root.is_root = true;
            new_root.values.insert(old_root_slot);
            new_root.values.insert(split_slot);

            self.root = Box::into_raw(Box::new(new_root));
        }
    }

    // TODO: Handle new leaves created from `find_child` match - ensure next nodes are set
    #[must_use]
    pub fn _insert(
        raw_node: *mut Node<K, V>,
        value: Slot<K, V>,
    ) -> Option<(Slot<K, V>, Slot<K, V>)> {
        let mut node = unsafe { &mut (*raw_node) };

        // If `split` is set, it will hold the updated slot for `node` and a new slot for the
        // greater node
        let mut split = None;
        if node.almost_full() {
            let (gt_node, gtk) = node.split();

            // If the node was internal, then it needs the replacement key needs removed
            if !node.is_leaf() {
                let rk = node
                    .values
                    .last()
                    .map(|s| s.0)
                    .expect("there should be a last node after split");
                let rk_slot = Slot::new_internal(rk, ptr::null_mut());

                node.values.remove(&rk_slot);
            }

            split = node.get_separators(Some((gt_node, gtk)));

            let last = node
                .values
                .last()
                .map(|s| s.0)
                .expect("there should be a last node");

            // TODO: this makes returning get_separators later incorrect, should keep og node
            // around
            if value.0 > last {
                node = unsafe { &mut *gt_node };
            }

            // split = Some((gt_node, gtk));
        }

        let ptr = match node.find_child(value) {
            Some(ptr) => ptr,
            None if node.is_root || !node.is_leaf() => {
                // Figure out what type of node we need to create:
                let new = match node.values.first() {
                    Some(n) => match n.1 {
                        Either::Left(_) => unreachable!(),
                        Either::Right(ptr) => match unsafe { &(*ptr).t } {
                            NodeType::Internal => Node::new_internal(node.max),
                            NodeType::Leaf => Node::new_leaf(node.max),
                        },
                    },
                    None => Node::new_leaf(node.max),
                };

                let ptr = Box::into_raw(Box::new(new));
                let slot = Slot::new_internal(value.0 + 1, ptr);
                node.values.insert(slot);

                ptr
            }
            None => {
                node.values.replace(value);
                return split;
                // return node.get_separators(split);
            }
        };

        if let Some((replace_slot, new_slot)) = BTree::_insert(ptr, value) {
            node.values.replace(replace_slot); // Test this replaces rather than adds
            node.values.replace(new_slot);
        }

        // node.get_separators(split)
        split
    }

    pub fn get(&self, key: K) -> Option<Slot<K, V>> {
        if self.root.is_null() {
            return None;
        }

        let test = Slot::new_internal(key, ptr::null_mut());
        Self::_get(self.root, test)
    }

    fn _get(raw_node: *mut Node<K, V>, slot: Slot<K, V>) -> Option<Slot<K, V>> {
        let node = unsafe { &(*raw_node) };

        match node.find_child(slot) {
            Some(ptr) => Self::_get(ptr, slot),
            None if node.is_leaf() => {
                return match node.values.get(&slot) {
                    Some(slot) => Some(*slot),
                    None => None,
                }
            }
            None => None,
        }
    }

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

#[cfg(test)]
mod test {
    use std::ops::Range;

    use super::*;

    fn get_inserts(key_range: Range<u8>) -> Vec<(u8, u8)> {
        let mut ret = Vec::with_capacity(key_range.len());

        let mut keys = key_range.collect::<Vec<u8>>();
        keys.shuffle(&mut thread_rng());

        for key in keys {
            let value = key + 10;
            ret.push((key, value));
        }

        ret
    }

    #[test]
    fn test_btree() {
        const MAX: usize = 8;

        let mut tree = BTree::new(MAX);

        // let inserts = get_inserts(0..10);
        let inserts = get_inserts(0..6);
        for (k, v) in &inserts {
            eprintln!("inserting {k}:{v}");
            tree.insert(Slot::new_leaf(*k, *v));
        }

        BTree::print(tree.root);

        for (k, v) in inserts {
            let test = match tree.get(k) {
                Some(t) => t,
                None => panic!("Could not find {k}:{v}"),
            };

            let got = get_left!(test);
            assert!(got == v, "Expected: {v}\n     Got {got}");
        }
    }
}
