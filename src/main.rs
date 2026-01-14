#![cfg_attr(debug_assertions, allow(dead_code, unused_imports))]

use std::{
    error::Error,
    io::{self, Write},
    process::Stdio,
};

use crate::{cursor::Cursor, read_seek_file::ReadSeekFile};

mod cache_file;
mod char_cursor_ext;
mod cursor;
mod memory_file;
mod read_seek_file;
mod span;
mod utf8_file;

fn main() -> Result<(), Box<dyn Error>> {
    let file = ReadSeekFile::from(std::fs::File::open("test_file.alm")?);
    let mut head = file.start()?;

    while let Some(cursor) = head {
        let data = cursor.data()?;
        io::stdout().flush()?;
        print!("{data:02X}");
        head = cursor.seek(cursor::Seek::Right(1))?;
    }

    // let byte_file = CachedReadFile::from(std::fs::File::open("test_file.alm")?);
    // let utf8_file = UTF8File::from(byte_file);
    // let mut head = utf8_file.start()?;
    //
    // while let Some(cursor) = head {
    //     let data = cursor.data()?;
    //
    //     print!("{data:?}");
    //
    //     head = cursor.next()?;
    // }
    //
    Ok(())
}
