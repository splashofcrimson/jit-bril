#![feature(proc_macro_hygiene)]
use compiler::Compiler;
use interpreter::Interpreter;

use std::{env, process, io::{self, Read}};

mod compiler;
mod interpreter;
mod program;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Expected one argument");
        process::exit(1);
    }
    let bril_ir = match program::read_json(&args[2]) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{}", e);
            eprintln!("Couldn't parse Bril file");
            process::exit(1);
        }
    };

    let mode = &args[1];

    if mode == "interp" {
        let interpreter = Interpreter::new(&bril_ir);
        interpreter.eval_program();
    }
    else if mode == "jit" {
        let mut compiler = Compiler::new(bril_ir);
        let main_idx: i64 = *compiler.index_map.get("main").unwrap();
        compiler.compile_and_run(main_idx);
    }
    else {
        eprintln!("Invalid mode");
    }

}

