#![feature(proc_macro_hygiene)]
use compiler::Compiler;
// use interpreter::Interpreter;
use jit::Interpreter;

use std::{
    env,
    io::{self, Read},
    process,
};

mod compiler;
mod interpreter;
mod jit;
mod program;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Expected at least one argument");
        process::exit(1);
    }

    let mut jit = false;
    let mut jit_n = 0;
    let mut osr = false;
    let mut osr_n = 0;
    let mut file_name = "";
    let mut cli_args = Vec::<i64>::new();
    let mut skip = false;
    for i in 1..args.len() {
        if skip {
            skip = false;
            continue;
        }
        match args[i].as_ref() {
            "-jit" => {
                jit = true;
                if i + 1 == args.len() {
                    eprintln!("Expected argument for -jit");
                    process::exit(1);
                } else {
                    jit_n = args[i + 1].parse().unwrap();
                    skip = true;
                }
            }
            "-osr" => {
                osr = true;
                if i + 1 == args.len() {
                    eprintln!("Expected argument for -jit");
                    process::exit(1);
                } else {
                    osr_n = args[i + 1].parse().unwrap();
                    skip = true;
                }
            }
            _ => {
                if file_name == "" {
                    file_name = &args[i];
                } else {
                    println!("{}", args[i]);
                    cli_args.push(args[i].parse().unwrap());
                }
            }
        }
    }

    let bril_ir = match program::read_json(file_name) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{}", e);
            eprintln!("Couldn't parse Bril file");
            process::exit(1);
        }
    };

    let mut interpreter = Interpreter::new(&bril_ir, jit, jit_n, osr, osr_n);
    interpreter.eval_program(cli_args);
}
