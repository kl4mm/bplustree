pub mod btree;
pub mod node;
pub mod slot;

#[macro_export]
macro_rules! get_left {
    ( $slot:ident ) => {{
        match $slot.1 {
            Either::Left(l) => l,
            Either::Right(_) => unreachable!(),
        }
    }};
}

#[macro_export]
macro_rules! get_right {
    ( $slot:ident ) => {{
        match $slot.1 {
            Either::Left(_) => unreachable!(),
            Either::Right(r) => r,
        }
    }};
}
