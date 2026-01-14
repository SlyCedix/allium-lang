use crate::cursor::Cursor;

pub trait CharCursorExt: Cursor<Item = char> {
    fn lookahead_match(&self, pattern: &str) -> anyhow::Result<(bool, Option<Self>)>;
}

impl<C: Cursor<Item = char>> CharCursorExt for C {
    fn lookahead_match(&self, pattern: &str) -> anyhow::Result<(bool, Option<Self>)> {
        // weird order of operations here ensures we correctly return true if
        // a string terminates in <eof>, but all characters match
        let mut head = Some(self.clone());

        for char in pattern.chars() {
            // check for eof first
            let h = match head {
                Some(h) => h,
                None => return Ok((false, None)),
            };

            // then check validity in advance
            let data = h.data()?;
            if data != char {
                return Ok((false, None));
            }
            head = h.next()?;
        }

        Ok((true, head))
    }
}
