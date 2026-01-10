/// refers to a source of a cheaply clonable items
///
/// includes lifetime to ensure [Span] and [Cursor]r can store references to [File]
///
/// planned types of this are:
///  - [ ] `byte`
///  - [ ] `char`, interpretting File<byte> as utf-8
///  - [ ] `Token` interpretting Char in allium
///  - [ ] `TokenTree` interpretting TokenTree as a tree of blocks  
#[allow(refining_impl_trait)]
pub trait File<'a> {
    type Item: Sized + Clone;
    type Cursor: Cursor<'a, Item = Self::Item>;

    /// get the cursor associated with the start of this stream
    fn start(&'a self) -> anyhow::Result<Option<Self::Cursor>>;
}

/// Cheaply clonable struct which refers to a single value in a [`File`]
///
/// [`PartialEq`] and [`Eq`] should be implemented such that two [`Cursor`]s are equal if they
/// refer to the same location in the [`File`]
///
/// [`Clone`] should be implemented such that the resulting [`Cursor`] is equal to the original
/// reference
///
/// [`PartialOrd`] should be implmented such that a cursor in the same [`File`] is less than
/// another if successive calls to [`Cursor::next`] would eventually yield the other
pub trait Cursor<'a>: Sized + Clone + PartialEq + Eq + PartialOrd {
    type Item: Sized + Clone;
    type Span: Span<'a, Item = Self::Item>;

    /// get the value that this cursor refers to
    fn data(&self) -> anyhow::Result<Self::Item>;

    /// get the cursor immediately following this one, or `None``, indicating that this cursor is the
    /// final one in the stream.
    fn next(&self) -> anyhow::Result<Option<Self>>;

    /// get the span between `self` (inclusive) and `other` (non-inclusive)
    ///
    /// `self.span_to(self)` should result in a span with `len() == 1`
    fn span_to(&self, other: &Self) -> anyhow::Result<Self::Span>;
}

/// Cheaply clonable struct which refers to a range of values in a [`File`]
///
/// [`PartialEq`] and [`Eq`] should be implemented such that two [`Span`]s are equivalent if
/// they refer to the same range of [`File`]
///
/// [`Clone`] should be implemented such that the resulting [`Span`] would be equal to the original
/// reference
pub trait Span<'a>: Clone + PartialEq + Eq {
    type Item: Sized + Clone;

    /// get an iterator over the values within this span
    fn data(&self) -> anyhow::Result<impl Iterator<Item = anyhow::Result<Self::Item>>>;

    /// get the number of elements in this span
    fn len(&self) -> anyhow::Result<usize>;
}
