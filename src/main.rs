#![feature(proc_macro_hygiene)]
use compiler::Compiler;
use interpreter::Interpreter;

use std::{env, process, io::{self, Read}};

mod compiler;
mod interpreter;
mod program;

fn main() {
    let args: Vec<String> = env::args().collect();
    println!("{:?}", args);

    let mut buffer = String::new();
    let stdin = io::stdin();
    let mut handle = stdin.lock();
    let _ = handle.read_to_string(&mut buffer);
    let bril_ir = match serde_json::from_str(&buffer) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{}", e);
            eprintln!("Couldn't parse Bril file");
            process::exit(1);
        }
    };

    let mode;

    if args.len() == 1 {
        mode = "jit";
    } else {
        mode = &args[1];
    }

    if mode == "interp" {
        let interpreter = Interpreter::new(bril_ir);
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

