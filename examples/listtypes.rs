//! pretty-prints all the declared types in the
#![feature(io)]

extern crate elm_eureka;

use std::io::prelude::*;
use std::io::BufReader;
use std::fs::File;
use std::env::args;

use elm_eureka::lexer::Lexer;
use elm_eureka::parser::Parser;
use elm_eureka::ast::{Type,Record};

trait Descriptible {
    fn describe(&self) -> String;
}

impl<T:Descriptible> Descriptible for [T] {
    fn describe(&self) -> String {
        match self.len() {
            0 => format!("nothing"),
            1 => self[0].describe(),
            2 => format!("{} and {}", self[0].describe(), self[1].describe()),
            _ => {
                let (head, tail) = self.split_first().unwrap();
                format!("{}, {}", head.describe(), tail.describe())
            },
        }
    }
}

impl Descriptible for (String, Type) {
    fn describe(&self) -> String {
        let &(ref name, ref type_) = self;
        format!("a field {} is {}", name, type_.describe())
    }
}

impl Descriptible for Record {
    fn describe(&self) -> String {
        let variable_descr =
            match self.variable_over {
                Some(ref name) => format!("generic over {} and", name),
                None => format!(""),
            };
        format!("a record {} with {}", variable_descr, self.fields.describe())
    }
}

impl Descriptible for Type {
    fn describe(&self) -> String {
        match *self {
            Type::Terminal(ref name) =>
                format!("the type {}", name),
            Type::Variable(ref name) =>
                format!("a type variable {}", name),
            Type::Application(ref name,ref arguments) =>
                format!(
                    "the higher-kinded type {} to which are applied {}",
                    name,
                    arguments.describe()
                ),
            Type::UnitType => format!("the unit type"),
            Type::EmptyRecord => format!("an empty record"),
            Type::Function(_,_) => format!("a function"),
            Type::Record(ref record) => record.describe(),
            Type::Tuple(ref contained) =>
                format!("a tuple containing {}", contained.describe()),
        }
    }
}

pub fn main() {
    let file_to_read =
        args().nth(1).unwrap_or(String::from("examples/elmjutsu-5k.elm"));
    let file = File::open(file_to_read).unwrap();
    let reader = BufReader::new(file);
    let lex = Lexer::new(reader.chars().map( |x| x.unwrap() ));
    let parser = Parser::new(lex);
    let (aliases, declars) = parser.get_types();
    for type_alias in aliases {
        println!("type alias {}, is:\n  {}",type_alias.name, type_alias.type_.describe());
    }
    for type_declr in declars {
        println!("type {}, is:",type_declr.name);
        for &(ref alt_name, ref alt_type) in type_declr.alternatives.iter() {
            println!("  {} of {}", alt_name, alt_type.describe());
        }
    }
}