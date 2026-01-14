mod variants;

use std::marker::PhantomData;

pub use variants::*;

use crate::cursor::Cursor;

#[derive(Clone)]
pub struct Punct(char);

#[derive(Clone)]
pub enum Tok {
    Whitespace(Whitespace),
    Identifier(Identifier),
    Literal(Literal),
    Punct(Punct),
}

/// The result of a [`Parse::parse`] operation
pub enum Munched<Token, Cursor> {
    /// Indicates that the parse operation succeeded and produced a `Token` as well as the next
    /// cursor, if one exists
    Some(Token, Option<Cursor>),
    /// Indicates that the parse operation failed due to an error in the input with a short string
    /// explaining why. 
    ///
    /// We intentionally do not bring in any explicit error type since this message should either:
    ///  - contain only a short, one line description about what error occurred, to be prettified by an
    ///  outer function
    ///
    /// **remarks:** do not use this to bubble errors produced by [`anyhow`], instead this should 
    /// be used exclusively to communicate that an error has occurred in the process of parsing, e.g.:
    ///     - invalid character
    ///     - unexpected <eof>
    ///     - unterminated literal
    ///
    /// **remarks:** may be shadowed as parsing of other tokens continues, if something else
    /// succeeded
    Err(String),
    /// Indicates that no error occurred, but no valid token was created
    None,
}

/// represents an object which "munches" on a [`Cursor`] stream
///
/// Implemented extremely generically because constraints 
pub trait Munch {
    type Token;
    type Cursor;

    fn munch(&self, cursor: &Self::Cursor) -> anyhow::Result<Munched<Self::Token, Self::Cursor>>;
}
