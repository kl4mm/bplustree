use crate::node::Node;

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum Either<A, B> {
    Left(A),
    Right(B),
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub struct Slot<A, B>(pub A, pub Either<B, *mut Node<A, B>>);

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

#[cfg(test)]
mod test {
    use std::collections::BTreeSet;

    use super::Slot;

    #[test]
    fn test_set() {
        let mut slots = BTreeSet::new();
        for (a, b) in (0..10).zip((100..200).step_by(10)) {
            slots.insert(Slot::new_leaf(a, b));
        }

        let want_len = slots.len();
        let mut want = Vec::new();

        for (a, b) in (0..10).zip((200..300).step_by(10)) {
            let slot = Slot::new_leaf(a, b);
            slots.replace(slot);
            want.push(slot);
        }

        let have_len = slots.len();
        assert!(want_len == have_len, "\nWant: {:?}\nHave: {:?}\n", want_len, have_len);

        let have = slots.iter().map(|s| *s).collect::<Vec<Slot<i32, i32>>>();
        assert!(want == have, "\nWant: {:?}\nHave: {:?}\n", want, have);
    }
}
