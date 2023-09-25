use std::{collections::BTreeSet, ptr};

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
enum Either<A, B> {
    Left(A),
    Right(B),
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
    values: BTreeSet<Slot<K, V>>,
    next: *mut Node<K, V>,
    max: usize,
}

pub struct Seperator<K, V> {
    k: K,
    ptr: *mut Node<K, V>,
}

pub type Seperators<K, V> = (Seperator<K, V>, Seperator<K, V>);

impl<K, V> Seperator<K, V> {
    pub fn new(k: K, ptr: *mut Node<K, V>) -> Self {
        Self { k, ptr }
    }
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

    pub fn insert(raw_node: *mut Node<K, V>, value: Slot<K, V>) -> Option<Seperators<K, V>> {
        assert!(!raw_node.is_null());

        let node = unsafe { &mut (*raw_node) };

        let split = match node.t {
            NodeType::Internal => 'leaf: {
                for slot in &node.values {
                    if value >= *slot {
                        let ptr = match slot.1 {
                            Either::Left(_) => unreachable!(),
                            Either::Right(ptr) => ptr,
                        };

                        // Will BTreeSet replace any existing slots if k == slot.k?
                        if let Some((sep_a, sep_b)) = Self::insert(ptr, value) {
                            node.values.replace(Slot::new_internal(sep_a.k, sep_a.ptr));
                            node.values.replace(Slot::new_internal(sep_b.k, sep_b.ptr));
                        }

                        break 'leaf node.almost_full();
                    }
                }

                // At this point insert a new leaf node, add slot to internal node, insert into
                // leaf
                let leaf: *mut Node<K, V> = Box::into_raw(Box::new(Node::new_leaf(node.max)));
                let internal_slot = Slot::new_internal(value.0, leaf);
                node.values.insert(internal_slot);

                // This should always return None since it's a new page
                Self::insert(leaf, value);

                node.almost_full()
            }
            NodeType::Leaf => 'leaf: {
                let last = node.values.iter().last();
                if last.is_none() || node.next.is_null() || value > *last.unwrap() {
                    node.values.insert(value);

                    break 'leaf node.almost_full();
                }

                // This should really only run when the structure is just a linked list. Once
                // internal nodes are added this should be unreachable, since the internal node
                // will direct to the correct leaf node where value > last is always true. Ignore
                // any seperators that come out of this:
                Self::insert(node.next, value);
                false
            }
        };

        if !split {
            return None;
        }

        // Create a new node of the same type, insert into both, return seperators to caller?
        let raw_new_node: *mut Node<K, V> = match node.t {
            NodeType::Internal => Box::into_raw(Box::new(Node::new_internal(node.max))),
            NodeType::Leaf => Box::into_raw(Box::new(Node::new_leaf(node.max))),
        };
        let new_node = unsafe { &mut (*raw_new_node) };

        let values = std::mem::take(&mut node.values);

        for slot in values.iter().take(node.max / 2) {
            node.values.insert(*slot);
        }
        for slot in values.iter().skip(node.max / 2) {
            new_node.values.insert(*slot);
        }

        // caller will reinsert, will know if split is needed
        Some((
            Seperator::new(node.values.last().unwrap().0, raw_node),
            Seperator::new(new_node.values.last().unwrap().0, raw_new_node),
        ))
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
        self.values.len() + 1 == self.max
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_btree() {
        const MAX: usize = 5;
        let root = Box::into_raw(Box::new(Node::new_leaf(MAX)));

        BTree::insert(root, Slot::new_leaf(1, 2));
        BTree::insert(root, Slot::new_leaf(3, 4));

        BTree::print(root);
    }
}
