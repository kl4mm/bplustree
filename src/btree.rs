use std::ptr;

use crate::get_right;
use crate::node::{Node, NodeType};
use crate::slot::{Either, Slot};

pub struct BTree<K, V> {
    root: *mut Node<K, V>,
    max: usize,
}

pub trait Increment {
    const MAX: Self;

    fn increment(&mut self);
    fn next(&self) -> Self;
}

macro_rules! impl_increment {
    ($( $t:ty ),*) => {
        $(
        impl Increment for $t {
            const MAX: Self = Self::MAX;

            fn increment(&mut self) {
                *self += 1;
            }

            fn next(&self) -> Self {
                self + 1
            }
        }
        )*
    };
}

impl_increment!(i8, i16, i32, i64, isize, u8, u16, u32, u64, usize);

use std::fmt::Debug;
impl<K, V> BTree<K, V>
where
    K: Clone + Copy + Debug + Ord + Copy + Increment,
    V: Clone + Copy + Debug + Eq,
{
    pub fn new(max: usize) -> Self {
        Self {
            root: ptr::null_mut(),
            max,
        }
    }

    /*
                  [255]
        [1, 2, 3, 4, 5, 6, 7, 8]
                    |
                    ▼
                [5, 255]
        [1, 2, 3, 4][5, 6, 7, 8]
    */
    pub fn insert(&mut self, entry: Slot<K, V>) {
        assert!(entry.is_leaf());

        if self.root.is_null() {
            let mut root = Node::new_leaf(self.max);
            root.is_root = true;
            self.root = Box::into_raw(Box::new(root));
        }

        if let Some((s, ptr)) = BTree::_insert(self.root, entry) {
            eprintln!("ROOT split: {:?} with {:?}", s, ptr);
            assert!(get_right!(s) == self.root);

            let root = unsafe { &mut *self.root };
            root.is_root = false;

            let mut node = Node::new_internal(self.max);
            node.is_root = true;
            node.replace(s);
            // new_root.replace(Slot::new_internal(K::MAX, ptr));

            // match node.values.iter().find(|s| get_right!(s) == ptr) {
            //     Some(s) => {
            //         node.values.replace(Slot::new_internal(s.0, ptr));
            //     }
            //     None => match node.values.replace(Slot::new_internal(K::MAX, ptr)) {
            //         Some(s) => {
            //             eprintln!("SLOT DISAPPEARING: {:?}", s);
            //         }
            //         None => {}
            //     },
            // };

            match node.values.iter().find(|s| get_right!(s) == ptr) {
                Some(s) => {
                    node.values.replace(Slot::new_internal(s.0, ptr));
                }
                None => match node.values.replace(Slot::new_internal(K::MAX, ptr)) {
                    Some(s) => {
                        let ptr = get_right!(s);
                        let dis = unsafe { &*ptr };
                        let ls = dis.values.last().unwrap();

                        let k = if dis.is_leaf() { ls.0.next() } else { ls.0 };
                        let s = Slot::new_internal(k, ptr);
                        match node.values.replace(s) {
                            Some(s) => {
                                eprintln!("SLOT DISAPPEARING: {:?}", s);
                            }
                            None => {}
                        }
                    }
                    None => {}
                },
            };

            self.root = Box::into_raw(Box::new(node));
        }
    }

    /// Returns a slot for the original page (lower half) and a pointer to the new page (higher
    /// half) if there is a split.
    #[must_use]
    pub fn _insert(
        raw_node: *mut Node<K, V>,
        value: Slot<K, V>,
    ) -> Option<(Slot<K, V>, *mut Node<K, V>)> {
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
                eprintln!("for {:?} with values {:?}", value, node.values);
                unreachable!();

                // // Figure out what type of node we need to create:
                // let new = match node.first_v() {
                //     Some(n) => match n {
                //         Either::Left(_) => unreachable!(),
                //         Either::Right(ptr) => match unsafe { &(*ptr).t } {
                //             NodeType::Internal => Node::new_internal(node.max),
                //             NodeType::Leaf => Node::new_leaf(node.max),
                //         },
                //     },
                //     None => Node::new_leaf(node.max),
                // };

                // let is_leaf = new.is_leaf();
                // let ptr = Box::into_raw(Box::new(new));
                // let slot = Slot::new_internal(value.0.next(), ptr);
                // node.insert(slot);

                // // Leaf next ptrs need to be set here since the algorithm is slightly wrong
                // // Ideally only split would set the next ptrs
                // if is_leaf && node.len() > 1 {
                //     // Find its sibling nodes
                //     let values = node.iter().collect::<Vec<&Slot<K, V>>>();
                //     let i = values.binary_search(&&slot).unwrap();

                //     if i > 0 {
                //         let l = i - 1;

                //         // Len is greater than 1 so this is ok to unwrap
                //         let slot = values.get(l).unwrap();
                //         // If `new` is a leaf node then `node` is internal, so we can take the ptr
                //         let left = unsafe { &mut *get_right!(slot) };

                //         left.next = ptr;
                //     }

                //     if i < values.len() - 1 {
                //         let r = i + 1;

                //         // Len is greater than 1 so this is ok to unwrap
                //         let slot = values.get(r).unwrap();
                //         // If `new` is a leaf node then `node` is internal, so we can take the ptr
                //         let right = get_right!(slot);

                //         let new = unsafe { &mut *ptr };
                //         new.next = right;
                //     }
                // }

                // ptr
            }
            None => {
                node.replace(value);
                // return node.get_separators(split, raw_node);
                return Node::get_separator(raw_node, split);
            }
        };

        if let Some((s, ptr)) = BTree::_insert(ptr, value) {
            eprintln!("split: {:?} with {:?}", s, ptr);
            node.replace(s);

            match node.values.iter().find(|s| get_right!(s) == ptr) {
                Some(s) => {
                    node.values.replace(Slot::new_internal(s.0, ptr));
                }
                None => match node.values.replace(Slot::new_internal(K::MAX, ptr)) {
                    Some(s) => {
                        let ptr = get_right!(s);
                        let dis = unsafe { &*ptr };
                        let ls = dis.values.last().unwrap();
                        let k = if dis.is_leaf() { ls.0.next() } else { ls.0 };
                        let s = Slot::new_internal(k, ptr);
                        match node.values.replace(s) {
                            Some(s) => {
                                eprintln!("SLOT DISAPPEARING: {:?}", s);
                            }
                            None => {}
                        }
                    }
                    None => {}
                },
            };

            // if node.values.len() == 1 {
            //     node.replace(Slot::new_internal(K::MAX, ptr));
            // }

            // // node.replace(os);
        }

        // node.get_separators(split, raw_node)
        Node::get_separator(raw_node, split)
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

        // let inserts = get_inserts(0..50);
        let inserts = get_inserts(0..10);
        for (k, v) in &inserts {
            eprintln!("inserting {} : {}", k, v);
            tree.insert(Slot::new_leaf(*k, *v));
        }

        eprintln!();
        Node::print(tree.root);

        for (k, v) in &inserts {
            let test = match tree.get(*k) {
                Some(t) => t,
                None => panic!("Could not find {k}:{v}"),
            };

            let got = get_left!(test);
            assert!(got == *v, "Expected: {v}\n     Got {got}");
        }

        // let (first_half, second_half) = inserts.split_at(inserts.len() / 2);

        // // Delete and make sure they no longer exist in the tree
        // for (k, _) in first_half {
        //     tree.delete(*k);
        // }
        // for (k, _) in first_half {
        //     match tree.get(*k) {
        //         Some(_) => panic!("Unexpected deleted key: {k}"),
        //         None => {}
        //     };
        // }

        // // Make sure keys can still be accessed
        // for (k, v) in second_half {
        //     let test = match tree.get(*k) {
        //         Some(t) => t,
        //         None => panic!("Could not find {k}:{v} in the second half"),
        //     };

        //     let got = get_left!(test);
        //     assert!(got == *v, "Expected: {v}\n     Got {got}");
        // }

        // // Insert a different range
        // let inserts = get_inserts(25..100);
        // for (k, v) in &inserts {
        //     tree.insert(Slot::new_leaf(*k, *v));
        // }

        // for (k, v) in &inserts {
        //     let test = match tree.get(*k) {
        //         Some(t) => t,
        //         None => panic!("Could not find {k}:{v}"),
        //     };

        //     let got = get_left!(test);
        //     assert!(got == *v, "Expected: {v}\n     Got {got}");
        // }
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
