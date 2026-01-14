use std::marker::PhantomData;

use unicode_id_start::{is_id_continue, is_id_start};

use crate::{
    char_cursor_ext::CharCursorExt,
    cursor::{Cursor, Seek},
    token::{Munch, Munched, Tok},
};

/// Any keyword or identifier-like token
#[derive(Debug, Clone)]
pub enum Identifier {
    /// Begins with either `_` or a character with the `XID_Start` unicode property
    /// After matching one such characters, continues collecting characters with the
    /// `XID_Continue` unicode property
    ///
    /// Inner string
    Standard(String),

    /// Any valid identifier preceeded by the raw specifier (`r#`)
    Raw(String),
}

pub struct MunchIdentifier<C> {
    _marker: PhantomData<C>,
}

impl<C> MunchIdentifier<C> {
    fn new() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

impl<C: Cursor<Item = char>> Munch for MunchIdentifier<C> {
    type Token = Tok;
    type Cursor = C;

    fn munch(&self, cursor: &Self::Cursor) -> anyhow::Result<Munched<Self::Token, Self::Cursor>> {
        let (is_raw, mut head) = match cursor.lookahead_match("r#")? {
            (true, Some(c)) => (true, Some(c)),
            (true, None) => {
                return Ok(Munched::Err(
                    "Failed to parse identifier: Found raw specifier but found <eof> after".into(),
                ));
            }
            (false, _) => (false, Some(cursor.clone())),
        };

        let data = head.as_ref().unwrap().data()?;

        if data != '_' && !is_id_start(data) {
            return Ok(Munched::None);
        }

        let mut out = String::new();

        while let Some(h) = head {
            let data = h.data()?;
            out.push(data);
            head = h.next()?;
            if !is_id_continue(data) {
                break;
            }
        }

        if is_raw {
            Ok(Munched::Some(Tok::Identifier(Identifier::Raw(out)), head))
        } else {
            Ok(Munched::Some(
                Tok::Identifier(Identifier::Standard(out)),
                head,
            ))
        }
    }
}
