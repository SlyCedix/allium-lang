#![cfg_attr(debug_assertions, allow(dead_code, unused_imports))]

use std::error::Error;

mod cached_read_file;
mod file;
mod utf8_file;

use crate::{
    cached_read_file::CachedReadFile,
    file::{Cursor, File},
    utf8_file::UTF8File,
};

fn main() -> Result<(), Box<dyn Error>> {
    let byte_file = CachedReadFile::from(std::fs::File::open("test_file.alm")?);
    let utf8_file = UTF8File::from(byte_file);
    let mut head = utf8_file.start()?;

    while let Some(cursor) = head {
        let data = cursor.data()?;

        print!("{data:?}");

        head = cursor.next()?;
    }

    Ok(())
}
