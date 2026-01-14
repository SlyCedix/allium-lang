use std::marker::PhantomData;

use crate::{
    char_cursor_ext::CharCursorExt,
    cursor::{Cursor, Seek},
    token::{Munch, Munched, Tok},
};

/// Any token which can be interpreted as whitespace
#[derive(Debug, Clone)]
pub enum Whitespace {
    /// contiguous sequence of characters contained within the Unicode Whitespace set:
    /// https://util.unicode.org/UnicodeJsps/list-unicodeset.jsp?a=[%3AWhitespace%3A]
    ///
    /// A whitespace token terminates at any line-feed (0x0A) character. If the proceeding line
    /// starts with more whitespace, as would be the case on a blank line or for indentation, a new
    /// whitespace token will begin.
    Standard(String),
    /// A line comment beginning with `//` and terminated at the next newline
    LineComment(String),
    /// A block comment beginning with `/*` and ending with `*/`.
    ///
    /// Block comments can be nested arbitrarilly deep, and will be parsed as a single token.
    /// As such, `/* /* */ *` would be parsed as a single [`Whitespace::BlockComment`]
    ///
    /// Block comment start and end characters may be escaped by preceeding the first character
    /// with a backslash (`\`)
    BlockComment(String),
}

pub struct MunchWhitespace<C> {
    _marker: PhantomData<C>,
}

impl<C> MunchWhitespace<C> {
    fn new() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

impl<C> Munch for MunchWhitespace<C>
where
    C: Cursor<Item = char>,
{
    type Token = Tok;
    type Cursor = C;

    fn munch(&self, cursor: &Self::Cursor) -> anyhow::Result<Munched<Self::Token, Self::Cursor>> {
        let mut errors = String::new();

        let res = Whitespace::parse_standard(cursor)?;
        if let Munched::Some(tok, next) = res {
            return Ok(Munched::Some(tok, next));
        } else if let Munched::Err(e) = res {
            if !errors.is_empty() {
                errors.push('\n');
            }
            errors.push_str(e.as_str());
        }

        let res = Whitespace::parse_line_comment(cursor)?;
        if let Munched::Some(tok, next) = res {
            return Ok(Munched::Some(tok, next));
        } else if let Munched::Err(e) = res {
            if !errors.is_empty() {
                errors.push('\n');
            }
            errors.push_str(e.as_str());
        }

        let res = Whitespace::parse_block_comment(cursor)?;
        if let Munched::Some(tok, next) = res {
            return Ok(Munched::Some(tok, next));
        } else if let Munched::Err(e) = res {
            if !errors.is_empty() {
                errors.push('\n');
            }
            errors.push_str(e.as_str());
        }

        Ok(Munched::None)
    }
}

impl Whitespace {
    fn parse_standard<C: Cursor<Item = char>>(cursor: &C) -> anyhow::Result<Munched<Tok, C>> {
        if !cursor.data()?.is_whitespace() {
            return Ok(Munched::None);
        }

        let mut out = String::new();

        let mut head = Some(cursor.clone());

        while let Some(h) = head {
            let data = h.data()?;
            out.push(h.data()?);
            head = h.next()?;
            if !data.is_whitespace() {
                break;
            }
        }

        // don't advance head, we're at first non-whitespace character
        Ok(Munched::Some(
            Tok::Whitespace(Whitespace::Standard(out)),
            head,
        ))
    }

    fn parse_line_comment<C: Cursor<Item = char>>(cursor: &C) -> anyhow::Result<Munched<Tok, C>> {
        if matches!(cursor.lookahead_match("//")?, (false, _)) {
            return Ok(Munched::None);
        }

        let mut out = String::new();

        let mut head = Some(cursor.clone());

        while let Some(h) = head {
            let data = h.data()?;
            out.push(data);
            head = h.next()?;
            if data == '\n' {
                break;
            }
        }

        Ok(Munched::Some(
            Tok::Whitespace(Whitespace::LineComment(out)),
            head,
        ))
    }

    fn parse_block_comment<C: Cursor<Item = char>>(cursor: &C) -> anyhow::Result<Munched<Tok, C>> {
        if matches!(cursor.lookahead_match("/*")?, (false, _)) {
            return Ok(Munched::None);
        }

        let mut out = String::new();
        let mut depth = 0usize;
        let mut head = Some(cursor.clone());
        while let Some(h) = head {
            if let (true, h) = h.lookahead_match("/*")? {
                head = h;
                depth += 1;
                out.push_str("/*");
            } else if let (true, h) = h.lookahead_match("*/")? {
                head = h;
                depth -= 1;
                out.push_str("*/")
            } else if let (true, h) = h.lookahead_match("\\/*")? {
                head = h;
                out.push_str("\\/*");
            } else if let (true, h) = h.lookahead_match("\\*/")? {
                head = h;
                out.push_str("\\*/");
            } else {
                out.push(h.data()?);
                head = h.next()?;
            }

            if depth == 0 {
                break;
            }
        }

        if depth != 0 {
            Ok(Munched::Err(
                "Failed to parse block comment: Unexpected <eof>".into(),
            ))
        } else {
            Ok(Munched::Some(
                Tok::Whitespace(Whitespace::BlockComment(out)),
                head,
            ))
        }
    }
}
