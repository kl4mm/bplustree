use std::{collections::BTreeSet, ptr};

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
enum Either<A, B> {
    Left(A),
    Right(B),
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

#[derive(PartialEq)]
pub enum NodeType {
    Internal,
    Leaf,
}

pub struct Node<K, V> {
    t: NodeType,
    values: BTreeSet<Slot<K, V>>,
    next: *mut Node<K, V>,
    max: usize,
}

impl<K, V> Node<K, V>
where
    K: Copy + Ord,
    V: Copy + Eq,
{
    pub fn new_leaf(max: usize) -> Self {
        Self {
            t: NodeType::Leaf,
            values: BTreeSet::new(),
            next: ptr::null_mut(),
            max,
        }
    }

    pub fn new_internal(max: usize) -> Self {
        Self {
            t: NodeType::Leaf,
            values: BTreeSet::new(),
            next: ptr::null_mut(),
            max,
        }
    }

    pub fn almost_full(&self) -> bool {
        self.values.len() >= self.max / 2
    }

    /// Returns greater half and new key for it
    pub fn split(&mut self) -> (*mut Node<K, V>, K) {
        let len = self.values.len();
        let mid = *self
            .values
            .iter()
            .nth(len / 2)
            .expect("there should be a mid value");

        let gt = self.values.split_off(&mid);
        let k = mid.0;

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

        (node, k)
    }

    /// Returns `None` if self is a leaf.
    pub fn find_child(&mut self, value: Slot<K, V>) -> Option<*mut Node<K, V>> {
        if self.is_leaf() {
            return None;
        }

        let n = self.values.iter().find(|n| value < **n).unwrap();
        Some(get_right!(n))
    }

    pub fn is_leaf(&self) -> bool {
        self.t == NodeType::Leaf
    }
}

pub struct BTree<K, V> {
    root: *mut Node<K, V>,
}

impl<K, V> BTree<K, V>
where
    K: Clone + Copy + Ord + Copy,
    V: Clone + Copy + Eq,
{
    pub fn new(root: Node<K, V>) -> Self {
        let root = Box::into_raw(Box::new(root));

        Self { root }
    }

    pub fn insert(
        raw_node: *mut Node<K, V>,
        value: Slot<K, V>,
    ) -> Option<(Slot<K, V>, Slot<K, V>)> {
        let mut node = unsafe { &mut (*raw_node) };

        // If `split` is set, it will hold the updated slot for `node` and a new slot for the
        // greater node
        let mut split = None;
        if node.almost_full() {
            let (gt_node, gt_k) = node.split();

            let replace_k = node
                .values
                .last()
                .expect("there should be a last node after split")
                .0;

            let new_slot = Slot::new_internal(gt_k, gt_node);
            let replace_slot = Slot::new_internal(replace_k, node);

            split = Some((replace_slot, new_slot));

            // If there was a split, check if the value needs to be inserted into the new node
            if value > replace_slot {
                node = unsafe { &mut (*get_right!(new_slot)) };
            }
        }

        let ptr = match node.find_child(value) {
            Some(ptr) => ptr,
            None => {
                node.values.replace(value);
                return split;
            }
        };

        if let Some((replace_slot, new_slot)) = BTree::insert(ptr, value) {
            node.values.replace(replace_slot); // Test this replaces rather than adds
            node.values.replace(new_slot);
        }

        split
    }

    pub fn print(node: *mut Node<K, V>)
    where
        K: std::fmt::Debug,
        V: std::fmt::Debug,
    {
        if node.is_null() {
            return;
        }

        let node = unsafe { &(*node) };
        match node.t {
            NodeType::Internal => {
                println!("Internal Node: {:?}", node.next);
                println!("Contents: {:?}", node.values);
                println!();

                for slot in &node.values {
                    match slot.1 {
                        Either::Left(_) => unreachable!(),
                        Either::Right(ptr) => Self::print(ptr),
                    }
                }
            }
            NodeType::Leaf => {
                println!("Leaf Node: Next: {:?}", node.next);
                println!("Contents: {:?}", node.values);
                println!()
            }
        }
    }

    pub fn print_list(node: *mut Node<K, V>)
    where
        K: std::fmt::Debug,
        V: std::fmt::Debug,
    {
        if node.is_null() {
            return;
        }

        let node = unsafe { &(*node) };
        match node.t {
            NodeType::Internal => {
                panic!("Expected leaf node for list print");
            }
            NodeType::Leaf => {
                println!("Leaf Node: Next: {:?}", node.next);
                println!("Contents: {:?}", node.values);
                println!();

                Self::print_list(node.next)
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_btree() {
        const MAX: usize = 4;
        let root = Box::into_raw(Box::new(Node::new_leaf(MAX)));

        for (k, v) in (0..10).zip(10..20) {
            BTree::insert(root, Slot::new_leaf(k, v));
        }

        BTree::print_list(root);
    }
}
