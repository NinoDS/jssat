//! A macro crate the parses ECMAScript Grammar Notation, and generates
//! structures.

#![feature(drain_filter)]

mod ast;
use ast::*;

mod codegen_rs;

pub fn generate(code: &str) -> String {
    eprintln!("parsing productions");
    let productions = Production::from_json(code);

    eprintln!("generating productions");
    codegen_rs::generate(productions)
}
