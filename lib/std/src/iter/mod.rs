//! Iterator traits and adapters.

mod traits;
mod adapters;
mod sources;
mod range;

pub use self::traits::{DoubleEndedIterator, ExactSizeIterator, Extend, FromIterator, FusedIterator, IntoIterator, Iterator, Product, Sum};
pub use self::adapters::{
    Chain, Cloned, Copied, Cycle, Enumerate, Filter, FilterMap, FlatMap, Flatten, Fuse, Inspect, Map, MapWhile, Peekable, Rev,
    Scan, Skip, SkipWhile, StepBy, Take, TakeWhile, Zip,
};
pub use self::sources::{empty, from_fn, once, once_with, repeat, repeat_n, repeat_with, successors, Empty, FromFn, Once, OnceWith, Repeat, RepeatN, RepeatWith, Successors};
