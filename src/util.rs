#[macro_export]
macro_rules! if_not_last {
    ( $iter:expr, $index:expr, $action:expr) => {
        if $index < $iter.len() - 1 {
            $action
        }
    };
}
pub use if_not_last;
