use std::ptr;

use crate::get_right;
use crate::node::{Node, NodeType};
use crate::slot::{Either, Slot};

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
            let mut root = Node::new_leaf(self.max);
            root.is_root = true;
            self.root = Box::into_raw(Box::new(root));
        }

        if let Some((old_root_slot, split_slot)) = BTree::_insert(self.root, entry) {
            assert!(get_right!(old_root_slot) == self.root);

            let root = unsafe { &mut *self.root };
            root.is_root = false;

            let mut new_root = Node::new_internal(self.max);
            new_root.is_root = true;
            new_root.insert(old_root_slot);
            new_root.insert(split_slot);

            self.root = Box::into_raw(Box::new(new_root));
        }
    }

    #[must_use]
    pub fn _insert(
        raw_node: *mut Node<K, V>,
        value: Slot<K, V>,
    ) -> Option<(Slot<K, V>, Slot<K, V>)> {
        let mut node = unsafe { &mut *raw_node };

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
            None if !node.is_leaf() => {
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
                node.insert(slot);

                // Leaf next ptrs need to be set here since the algorithm is slightly wrong
                // Ideally only split would set the next ptrs
                if is_leaf && node.len() > 1 {
                    // Find its sibling nodes
                    let values = node.iter().collect::<Vec<&Slot<K, V>>>();
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
                node.replace(value);
                return node.get_separators(split, raw_node);
            }
        };

        if let Some((replace_slot, new_slot)) = BTree::_insert(ptr, value) {
            node.replace(replace_slot); // Test this replaces rather than adds
            node.replace(new_slot);
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
        let node = unsafe { &*raw_node };

        match node.find_child(slot) {
            Some(ptr) => Self::_get(ptr, slot),
            None if node.is_leaf() => {
                return match node.get(&slot) {
                    Some(slot) => Some(*slot),
                    None => None,
                }
            }
            None => None,
        }
    }

    pub fn delete(&mut self, key: K) -> bool {
        if self.root.is_null() {
            return false;
        }

        let test = Slot::new_internal(key, ptr::null_mut());
        Self::_delete(self.root, test)
    }

    fn _delete(raw_node: *mut Node<K, V>, slot: Slot<K, V>) -> bool {
        let node = unsafe { &mut *raw_node };

        match node.find_child(slot) {
            Some(ptr) => Self::_delete(ptr, slot),
            None if node.is_leaf() => return node.delete(&slot),
            None => false,
        }
    }

    #[cfg(test)]
    fn get_leftmost_leaf(raw_node: *mut Node<K, V>) -> *mut Node<K, V> {
        let node = unsafe { &*raw_node };
        if node.is_leaf() {
            return raw_node;
        }

        let mut ret = ptr::null_mut();
        if let Some(slot) = node.first() {
            ret = Self::get_leftmost_leaf(get_right!(slot));
        }

        ret
    }
}

#[cfg(test)]
mod test {
    use std::ops::Range;

    use rand::{seq::SliceRandom, thread_rng};

    use crate::get_left;

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
            tree.insert(Slot::new_leaf(*k, *v));
        }

        for (k, v) in &inserts {
            let test = match tree.get(*k) {
                Some(t) => t,
                None => panic!("Could not find {k}:{v}"),
            };

            let got = get_left!(test);
            assert!(got == *v, "Expected: {v}\n     Got {got}");
        }

        let (first_half, second_half) = inserts.split_at(inserts.len() / 2);

        // Delete and make sure they no longer exist in the tree
        for (k, _) in first_half {
            tree.delete(*k);
        }
        for (k, _) in first_half {
            match tree.get(*k) {
                Some(_) => panic!("Unexpected deleted key: {k}"),
                None => {}
            };
        }

        // Make sure keys can still be accessed
        for (k, v) in second_half {
            let test = match tree.get(*k) {
                Some(t) => t,
                None => panic!("Could not find {k}:{v} in the second half"),
            };

            let got = get_left!(test);
            assert!(got == *v, "Expected: {v}\n     Got {got}");
        }

        // Insert a different range
        let inserts = get_inserts(25..100);
        for (k, v) in &inserts {
            tree.insert(Slot::new_leaf(*k, *v));
        }

        for (k, v) in &inserts {
            let test = match tree.get(*k) {
                Some(t) => t,
                None => panic!("Could not find {k}:{v}"),
            };

            let got = get_left!(test);
            assert!(got == *v, "Expected: {v}\n     Got {got}");
        }
    }

    #[test]
    #[ignore]
    fn test_btree_scan() {
        const MAX: usize = 8;

        let mut tree = BTree::new(MAX);

        let mut inserts = get_inserts(0..50);
        for (k, v) in &inserts {
            tree.insert(Slot::new_leaf(*k, *v));
        }

        inserts.sort_by(|(ka, _), (kb, _)| ka.cmp(kb));

        let mut values = Vec::with_capacity(inserts.len());
        let mut cur = BTree::get_leftmost_leaf(tree.root);

        while cur != ptr::null_mut() {
            let node = unsafe { &*cur };
            node.iter().for_each(|s| {
                values.push((s.0, get_left!(s)));
            });

            cur = node.next;
        }

        // Flakey
        eprintln!("inserts: {:?}\n values: {:?}", inserts, values);
        assert!(inserts == values);
    }
}
