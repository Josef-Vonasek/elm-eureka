
//! pretty-prints all the declared types in the
#![feature(io)]

extern crate elm_eureka;

use std::io::prelude::*;
use std::io::BufReader;
use std::fs::File;
use std::env::args;

use elm_eureka::lexer::Lexer;
use elm_eureka::parser::Parser;

pub fn main() {
    let file_to_read =
        args().nth(1).unwrap_or(String::from("examples/elmjutsu-5k.elm"));
    let file = File::open(file_to_read).unwrap();
    let reader = BufReader::new(file);
    let lex = Lexer::new(reader.chars().map( |x| x.unwrap() ));
    let parser = Parser::new(lex);
    println!("{:#?}", parser);
}
