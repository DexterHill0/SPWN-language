use std::{fs, path::PathBuf};
mod compiler;
mod docgen;
mod error;
mod interpreter;
mod leveldata;
mod parser;
mod sources;

use std::io::{self, Write};

use compiler::compiler::{Compiler, Scope};
use interpreter::interpreter::{execute_code, Globals};
use parser::ast::ASTData;
use parser::parser::Parser;
use sources::SpwnSource;

use docgen::docgen::parse_doc_comments;

use crate::leveldata::object_data::serialize_obj;

// use ahash::AHashMap;
// use compiler::compiler::{Compiler, Scope};
// use interpreter::contexts::{Context, FullContext};
// use interpreter::interpreter::{execute_code, Globals};
// use parser::lexer::lex;
// use parser::parser::{parse, ASTData, ParseData};
// use slotmap::SlotMap;
// use sources::SpwnSource;

// fn run(code: String, source: SpwnSource) {
//     let tokens = lex(code);

//     let mut ast_data = ASTData::default();
//     let parse_data = ParseData {
//         source: source.clone(),
//         tokens,
//     };

//     let ast = parse(&parse_data, &mut ast_data);

//     match ast {
//         Ok(stmts) => {
//             ast_data.debug(&stmts);

//             let mut compiler = Compiler::new(ast_data);
//             compiler.code.instructions.push((vec![], vec![]));

//             let mut base_scope = compiler.scopes.insert(Scope::base());

//             match compiler.compile_stmts(stmts, base_scope, 0) {
//                 Ok(_) => {
//                     compiler.code.debug();

//                     // let bytes = to_bytes(&compiler.code);
//                     // println!("bytes: {}", bytes.len());

//                     // let mut file = File::create("test.spwnc").unwrap();
//                     // file.write_all(&bytes).unwrap();

//                     // let compressed = lz4_compression::prelude::compress(&bytes);
//                     // println!(
//                     //     "lz4 bytes: {}, {:.2}%",
//                     //     compressed.len(),
//                     //     (compressed.len() as f64) / (bytes.len() as f64) * 100.0
//                     // );

//                     // let compressed =
//                     //     yazi::compress(&bytes, yazi::Format::Raw, yazi::CompressionLevel::BestSize)
//                     //         .unwrap();
//                     // println!(
//                     //     "zlib bytes: {}, {:.2}%",
//                     //     compressed.len(),
//                     //     (compressed.len() as f64) / (bytes.len() as f64) * 100.0
//                     // );

//                     // println!("{:?}", bytes);

//                     // let mut globals = Globals {
//                     //     memory: SlotMap::default(),
//                     //     contexts: FullContext::Split(
//                     //         Box::new(FullContext::single(compiler.code.var_count)),
//                     //         Box::new(FullContext::single(compiler.code.var_count)),
//                     //     ),
//                     // };

//                     let mut globals = Globals::new();
//                     globals.init();

//                     if let Err(e) = execute_code(&mut globals, &compiler.code) {
//                         e.raise(source, &globals);
//                     }
//                 }
//                 Err(e) => e.raise(source),
//             }
//         }
//         Err(e) => {
//             e.raise(source);
//         }
//     }
// }

fn run_spwn(code: String, source: SpwnSource, doctest: bool) {
    if doctest {
        parse_doc_comments(code.clone());
        return;
    }

    let mut parser = Parser::new(&code, source.clone());

    let mut ast_data = ASTData::default();

    match parser.parse(&mut ast_data) {
        Ok(stmts) => {
            ast_data.debug(&stmts);

            let mut compiler = Compiler::new(ast_data, source.clone());

            match compiler.start_compile(stmts) {
                Ok(_) => {
                    compiler.code.debug();

                    let mut globals = Globals::new();
                    globals.init();

                    println!("\n\n\n");

                    if let Err(e) = execute_code(&mut globals, &compiler.code) {
                        e.raise(&code, source, &globals);
                    }

                    println!("Triggers:");
                    for trigger in globals.triggers.iter() {
                        println!("{}", serialize_obj(trigger.clone()));
                    }
                }
                Err(e) => e.raise(&code, source),
            }
        }
        Err(e) => e.raise(&code, source),
    }
}

fn main() {
    print!("\x1B[2J\x1B[1;1H");

    io::stdout().flush().unwrap();

    let file = std::env::args().nth(1).expect("no filename given");
    let doctest: bool = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "false".to_string())
        .parse()
        .expect("expected bool for doctest");
    let buf = PathBuf::from(file);

    let code = fs::read_to_string(&buf).unwrap();
    run_spwn(code, Some(buf), doctest);
    // println!("{}", std::mem::size_of::<Instruction>());
}
