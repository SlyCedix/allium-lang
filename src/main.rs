use std::error::Error;

mod cached_read_file;
mod utf8_file;
mod file;

use crate::{
    cached_read_file::CachedReadFile,
    file::{Cursor, File, Span},
};

fn main() -> Result<(), Box<dyn Error>> {
    // let file = CachedReadFile::from(std::fs::File::open("test_file.alm")?);
    // let mut head = file.start()?;
    //
    // loop {
    //     print!("{:#02X} ", head.data()?);
    //
    //     head = match head.next()? {
    //         Some(c) => c,
    //         None => break,
    //     }
    // }

    Ok(())
}
