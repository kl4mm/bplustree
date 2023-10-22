use std::ops::AddAssign;

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

    pub fn incr_k(&mut self)
    where
        A: AddAssign<u8>,
    {
        self.0 += 1;
    }
}
