#![feature(proc_macro_hygiene)]

extern crate dynasmrt;
extern crate dynasm;

use dynasm::dynasm;
use dynasmrt::{DynasmApi, DynasmLabelApi};

use std::{env, mem, process};
use std::collections::HashMap;

mod program;

struct AsmProgram {
    code: dynasmrt::ExecutableBuffer,
    start: dynasmrt::AssemblyOffset,
}

impl AsmProgram {
    fn compile(bril_func: &program::Function) -> AsmProgram {

        let mut var_offsets = HashMap::<String, i32>::new();
        let mut num_vars = 1;

        for inst in &bril_func.instrs {
            if let Some(dest) = &inst.dest {
                if !var_offsets.contains_key(dest) {
                    var_offsets.insert(dest.to_string(), -8 * num_vars);
                    num_vars += 1;
                }
            }
        }

        let num_bytes = if num_vars % 2 == 0 {
            8 * num_vars
        } else {
            16 * (num_vars / 2 + 1)
        };

        let mut asm = dynasmrt::x64::Assembler::new().unwrap();

        let start = asm.offset();

        dynasm!(asm
            // prologue
            ; push rbp
            ; mov rbp, rsp
            ; sub rsp, num_bytes
        );

        for inst in &bril_func.instrs {
            match inst.op.as_ref() {
                "add" => {
                    if let (Some(args), Some(dest)) = (&inst.args, &inst.dest) {
                        if let (Some(a), Some(b), Some(d)) = (var_offsets.get(&args[0]), var_offsets.get(&args[1]), var_offsets.get(dest)) {
                            dynasm!(asm
                                ; mov rax, [rbp + *a]
                                ; add rax, [rbp + *b]
                                ; mov [rbp + *d], rax
                            );
                        }
                    }
                }
                "const" => {
                    if let (Some(dest), Some(value)) = (&inst.dest, &inst.value) {
                        if let Some(d) = var_offsets.get(dest) {
                            dynasm!(asm
                                ; mov rax, *value
                                ; mov [rbp + *d], rax
                            );
                        }
                    }
                }
                "print" => {
                    if let Some(args) = &inst.args {
                        for arg in args {
                            if let Some(a) = var_offsets.get(arg) {
                                dynasm!(asm
                                    ; mov rdi, [rbp + *a]
                                    ; mov rax, QWORD print_int as _
                                    ; call rax
                                );
                            }
                        }
                    }
                }
                "nop" => { dynasm!(asm ; nop); }
                _ => { }
            }
        }

        // epilogue
        dynasm!(asm
            ; mov rsp, rbp
            ; pop rbp
            ; ret
        );

        let code = asm.finalize().unwrap();
        return AsmProgram {code: code, start: start};
    }

    fn run(self) -> bool {
        let f: extern "stdcall" fn() -> bool = unsafe {
            mem::transmute(self.code.ptr(self.start))
        };
        return f();
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("expected one argument");
        process::exit(1);
    }
    let bril_program = match program::read_json(&args[1]) {
        Ok(p) => p,
        Err(_) => {
            eprintln!("couldn't parse Bril file");
            process::exit(1);
        }
    };
    let asm_program = AsmProgram::compile(&bril_program.functions[0]);
    println!("{}", asm_program.run());
}

fn print_int(i: i64) {
    println!("{}", i);
}

