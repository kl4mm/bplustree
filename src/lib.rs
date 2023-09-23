use std::{collections::BinaryHeap, ptr};

#[derive(PartialEq, Eq, Debug)]
enum Either<A, B> {
    Left(A),
    Right(B),
}

#[derive(PartialEq, Eq, Debug)]
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
}

pub struct BTree<K, V> {
    root: *mut Node<K, V>,
}

#[derive(PartialEq)]
pub enum NodeType {
    Internal,
    Leaf,
}

pub struct Node<K, V> {
    t: NodeType,
    values: BinaryHeap<Slot<K, V>>,
    next: *mut Node<K, V>,
}

impl<K, V> BTree<K, V>
where
    K: Ord + Copy,
    V: Eq,
{
    pub fn new(root: Node<K, V>) -> Self {
        let root = Box::into_raw(Box::new(root));

        Self { root }
    }

    pub fn insert(node: *mut Node<K, V>, value: Slot<K, V>) {
        assert!(!node.is_null());

        let node = unsafe { &mut (*node) };

        match node.t {
            NodeType::Internal => {
                for slot in &node.values {
                    if value >= *slot {
                        let ptr = match slot.1 {
                            Either::Left(_) => unreachable!(),
                            Either::Right(ptr) => ptr,
                        };

                        return Self::insert(ptr, value);
                    }
                }

                // At this point insert a new leaf node, add slot to internal node, insert into
                // leaf
                let leaf: *mut Node<K, V> = Box::into_raw(Box::new(Node::new_leaf()));
                let internal_slot = Slot::new_internal(value.0, leaf);
                node.values.push(internal_slot);
                Self::insert(leaf, value)
            }
            NodeType::Leaf => {
                let last = node.values.iter().last();
                if last.is_none() || node.next.is_null() || value > *last.unwrap() {
                    node.values.push(value);
                    return;
                }

                Self::insert(node.next, value)
            }
        }
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
}

impl<K, V> Node<K, V>
where
    K: Ord,
    V: Eq,
{
    pub fn new_leaf() -> Self {
        Self {
            t: NodeType::Leaf,
            values: BinaryHeap::new(),
            next: ptr::null_mut(),
        }
    }

    pub fn new_internal() -> Self {
        Self {
            t: NodeType::Leaf,
            values: BinaryHeap::new(),
            next: ptr::null_mut(),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_btree() {
        let root = Box::into_raw(Box::new(Node::new_leaf()));

        BTree::insert(root, Slot::new_leaf(1, 2));
        BTree::insert(root, Slot::new_leaf(3, 4));

        BTree::print(root);
    }
}
