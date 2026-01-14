use std::{cmp::Ordering, marker::PhantomData};

use crate::cursor::{Cursor, Seek};

pub struct Span<C> {
    start: C,
    end: C,
}

pub struct SpanIterator<C> {
    curr: C,
    end: C,
}

pub trait SpanTo: Cursor + PartialOrd {
    /// Create a  [`Span`] between `self` and `other`
    ///
    /// Returns an error if the cursors cannot be compared with [`PartialOrd`] or if `other` occurs
    /// before `self`
    fn span_to(&self, other: &Self) -> anyhow::Result<Span<Self>> {
        match self.partial_cmp(other) {
            Some(Ordering::Less | Ordering::Equal) => Ok(Span {
                start: self.clone(),
                end: other.clone(),
            }),
            Some(_) => Err(anyhow::anyhow!(
                "Failed to create span: length must be greater than or equal to zero"
            )),
            None => Err(anyhow::anyhow!("Failed to create span: comparison failed")),
        }
    }
}

impl<C: Cursor + PartialOrd> SpanTo for C {}

impl<C: Cursor + PartialOrd> Iterator for SpanIterator<C> {
    type Item = anyhow::Result<C::Item>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.curr < self.end {
            let data = match self.curr.data() {
                Ok(d) => d,
                Err(e) => return Some(Err(e)),
            };

            self.curr = match self.curr.seek(Seek::Right(1)) {
                Ok(Some(c)) => c,
                Ok(None) => {
                    return Some(Err(anyhow::anyhow!("Reached <eof> while iterating span")));
                }
                Err(e) => return Some(Err(e)),
            };

            Some(Ok(data))
        } else {
            None
        }
    }
}

impl<C: Cursor> Span<C> {
    pub fn data(&self) -> anyhow::Result<SpanIterator<C>> {
        Ok(SpanIterator {
            curr: self.start.clone(),
            end: self.end.clone(),
        })
    }
}
