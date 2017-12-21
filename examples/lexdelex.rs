//! Pretty-prints the tokens created from lexing the elmjutsu-5k.elm file
#![feature(io)]

extern crate elm_eureka;

use std::io::prelude::*;
use std::io::BufReader;
use std::fs::File;

use elm_eureka::lexer::LexableIterator;

pub fn main() {
    let file = File::open("examples/elmjutsu-5k.elm").unwrap();
    let reader = BufReader::new(file);
    let lex = reader.chars().map( |x| x.unwrap() ).lex();

    for token in lex {
        print!("{} ", token);
    }
}