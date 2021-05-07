use std::{
    cmp::PartialOrd,
    ops::{
        Bound::{Excluded, Included, Unbounded},
        Range, RangeBounds, RangeFrom, RangeFull, RangeInclusive, RangeTo, RangeToInclusive,
    },
    rc::Rc,
};

/// This trait is used to describe constraints with different types.
pub trait InsideFunc<T> {
    /// Returns constraint as a function.
    fn contains_func(self) -> Rc<dyn Fn(&T) -> bool>;
}

impl<T: PartialEq + 'static> InsideFunc<T> for Vec<T> {
    fn contains_func(self) -> Rc<dyn Fn(&T) -> bool> {
        Rc::new(move |x| self.contains(x))
    }
}

macro_rules! impl_inside_func_for_arrays {
    ($($e:expr),*) => {$(
        impl<T: PartialEq + 'static> InsideFunc<T> for [T; $e] {
            fn contains_func(self) -> Rc<dyn Fn(&T) -> bool> {
                Rc::new(move |x| self.contains(x))
            }
        }
    )*}
}

impl_inside_func_for_arrays! {
    1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15,
    16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28,
    29, 30, 31, 32
}

fn range_contains_func<T, U>(range: U) -> Rc<dyn Fn(&T) -> bool>
where
    T: PartialOrd,
    U: RangeBounds<T> + 'static,
{
    Rc::new(move |x| {
        (match range.start_bound() {
            Included(ref start) => *start <= x,
            Excluded(ref start) => *start < x,
            Unbounded => true,
        }) && (match range.end_bound() {
            Included(ref end) => x <= *end,
            Excluded(ref end) => x < *end,
            Unbounded => true,
        })
    })
}

macro_rules! impl_inside_func_for_ranges {
    ($($t:ty),*) => {$(
        impl<T: PartialOrd + 'static> InsideFunc<T> for $t {
            fn contains_func(self) -> Rc<dyn Fn(&T) -> bool> {
                range_contains_func(self)
            }
        }
    )*}
}

impl_inside_func_for_ranges! {
    Range<T>, RangeInclusive<T>, RangeFrom<T>, RangeTo<T>, RangeToInclusive<T>, RangeFull
}
