use std::{fs::File, io::BufReader};

use crate::source::SourceFile;

mod lexer;
mod source;
mod error;
mod atom;

fn main() {
    let mut file = File::open("test_file.alm").unwrap();
    let mut reader = BufReader::new(file);

    let mut source = SourceFile::new("test_file.alm".to_string(), &mut reader).unwrap();
}
