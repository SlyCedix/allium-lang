use std::{cmp::Ordering, marker::PhantomData};

/// represents a seek operation for traversing a [`File`] with [`Cursor::seek`]
///
/// A given [`Cursor`] implementation may support any number
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Seek {
    /// Seek left-ward towards the beginning of the file
    Left(usize),
    /// Seek right-ward towards the end of the file
    Right(usize),
}

/// Cheaaply clonable representation of a single element in some stream of items.
pub trait Cursor: Clone + Sized {
    type Item;

    /// Get the data associated with this cursor, or an error indicating why this data could not be
    /// resolved
    ///
    /// No guarentee is made that this function is cheap to execute, in fact, as higher and higher
    /// orders of parser adapt of e.g. Bytes -> Chars -> Tokens -> SyntaxNodes -> AST, this call
    /// would typically descend all the way down to the lowest level and execute any necessary
    /// parsing on the fly. As a result, it is recommended that when the result of this function,
    /// is utilized it is either cached for local reuse, or proxied through a structure such as
    /// [`CacheFile`] in order to prevent unnecessary retraversal.
    ///
    /// Where possible, it is recommended practice that repeated calls to this function produce the
    /// same result, but there is no guarentee that this is the case
    fn data(&self) -> anyhow::Result<Self::Item>;

    /// Get a [`Cursor`] at a position relative to this one, or [`None`], indicating that no such
    /// cursor exists. If left seeking is supported, but seek would refer to memory further left than the
    /// start of the [`File`], this function is expected to return [`None`]. Unsupported
    /// seek operations are expected to error, rather than result in <eof>
    ///
    /// Similar to [`Cursor::data`], there is no guarentee that this is a cheap operation and, as is
    /// the case for most [`File`] implementations, may require iterating and parsing each element between
    /// `self` and the return value
    fn seek(&self, op: Seek) -> anyhow::Result<Option<Self>>;

    fn next(&self) -> anyhow::Result<Option<Self>> {
        self.seek(Seek::Right(1))
    }
}
