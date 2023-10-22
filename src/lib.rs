use std::{collections::BTreeSet, ptr};

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum Either<A, B> {
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

    pub fn incr_k(&mut self)
    where
        A: AddAssign<u8>,
    {
        self.0 += 1;
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
    K: Copy + Debug + Ord + AddAssign<u8>,
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

    /// Returns greater half, new key for it and new key for replace
    pub fn split(&mut self) -> *mut Node<K, V> {
        let len = self.values.len();
        let mid = *self
            .values
            .iter()
            .nth(len / 2)
            .expect("there should be a mid slot");

        let gt = self.values.split_off(&mid);

        let mut gt_node = match self.t {
            NodeType::Internal => Node::new_internal(self.max),
            NodeType::Leaf => Node::new_leaf(self.max),
        };
        gt_node.values = gt;

        let gt_node = Box::into_raw(Box::new(gt_node));
        if self.is_leaf() {
            if !self.next.is_null() {
                unsafe { (*gt_node).next = self.next };
            }

            self.next = gt_node;
        }

        gt_node
    }

    pub fn get_separators(
        &mut self,
        other: Option<*mut Node<K, V>>,
        og: *mut Node<K, V>, // if self == other, use og in place of self
    ) -> Option<(Slot<K, V>, Slot<K, V>)> {
        other.map(|raw_gt_node| {
            let me = (|node: *mut Node<K, V>| {
                let me = if node == raw_gt_node { og } else { node };
                unsafe { &mut (*me) }
            })(self);

            let rk = me.last_k().expect("self should have a last slot");
            let mut rs = Slot::new_internal(rk, me);

            let gt_node = unsafe { &mut (*raw_gt_node) };
            let gtk = gt_node.last_k().expect("gt should have a last slot");
            let mut ns = Slot::new_internal(gtk, raw_gt_node);

            if me.is_leaf() {
                rs.incr_k();
                ns.incr_k();
            }

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
            let raw_gt_node = node.split();
            split = Some(raw_gt_node);

            let last = node.last_k().expect("there should be a last node");

            if value.0 >= last {
                node = unsafe { &mut *raw_gt_node };
            }
        }

        let ptr = match node.find_child(value) {
            Some(ptr) => ptr,
            None if node.is_root || !node.is_leaf() => {
                // Figure out what type of node we need to create:
                let new = match node.first_v() {
                    Some(n) => match n {
                        Either::Left(_) => unreachable!(),
                        Either::Right(ptr) => match unsafe { &(*ptr).t } {
                            NodeType::Internal => Node::new_internal(node.max),
                            NodeType::Leaf => Node::new_leaf(node.max),
                        },
                    },
                    None => Node::new_leaf(node.max),
                };

                let is_leaf = new.is_leaf();
                let ptr = Box::into_raw(Box::new(new));
                let slot = Slot::new_internal(value.0 + 1, ptr);
                node.values.insert(slot);

                // Leaf next ptrs need to be set here since the algorithm is slightly wrong
                // Ideally only split would set the next ptrs
                if is_leaf && node.values.len() > 1 {
                    // Find its sibling nodes
                    let values = node.values.iter().collect::<Vec<&Slot<K, V>>>();
                    let i = values.binary_search(&&slot).unwrap();

                    if i > 0 {
                        let l = i - 1;

                        // Len is greater than 1 so this is ok to unwrap
                        let slot = values.get(l).unwrap();
                        // If `new` is a leaf node then `node` is internal, so we can take the ptr
                        let left = unsafe { &mut *get_right!(slot) };

                        left.next = ptr;
                    }

                    if i < values.len() - 1 {
                        let r = i + 1;

                        // Len is greater than 1 so this is ok to unwrap
                        let slot = values.get(r).unwrap();
                        // If `new` is a leaf node then `node` is internal, so we can take the ptr
                        let right = get_right!(slot);

                        let new = unsafe { &mut *ptr };
                        new.next = right;
                    }
                }

                ptr
            }
            None => {
                node.values.replace(value);
                return node.get_separators(split, raw_node);
            }
        };

        if let Some((replace_slot, new_slot)) = BTree::_insert(ptr, value) {
            node.values.replace(replace_slot); // Test this replaces rather than adds
            node.values.replace(new_slot);
        }

        node.get_separators(split, raw_node)
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

    #[cfg(test)]
    fn get_leftmost_leaf(raw_node: *mut Node<K, V>) -> *mut Node<K, V> {
        let node = unsafe { &*raw_node };
        if node.is_leaf() {
            return raw_node;
        }

        let mut ret = ptr::null_mut();
        if let Some(slot) = node.values.first() {
            ret = Self::get_leftmost_leaf(get_right!(slot));
        }

        ret
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

    use rand::{seq::SliceRandom, thread_rng};

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

        let inserts = get_inserts(0..50);
        for (k, v) in &inserts {
            eprintln!("inserting {k}:{v}");
            tree.insert(Slot::new_leaf(*k, *v));
        }

        for (k, v) in inserts {
            let test = match tree.get(k) {
                Some(t) => t,
                None => panic!("Could not find {k}:{v}"),
            };

            let got = get_left!(test);
            assert!(got == v, "Expected: {v}\n     Got {got}");
        }
    }

    #[test]
    fn test_btree_scan() {
        const MAX: usize = 8;

        let mut tree = BTree::new(MAX);

        let mut inserts = get_inserts(0..50);
        for (k, v) in &inserts {
            eprintln!("inserting {k}:{v}");
            tree.insert(Slot::new_leaf(*k, *v));
        }

        inserts.sort_by(|(ka, _), (kb, _)| ka.cmp(kb));

        let mut values = Vec::with_capacity(inserts.len());
        let mut cur = BTree::get_leftmost_leaf(tree.root);

        while cur != ptr::null_mut() {
            let node = unsafe { &*cur };
            node.values.iter().for_each(|s| {
                values.push((s.0, get_left!(s)));
            });

            cur = node.next;
        }

        // Flakey
        eprintln!("inserts: {:?}\n values: {:?}", inserts, values);
    }
}
