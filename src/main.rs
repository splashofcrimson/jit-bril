#![feature(proc_macro_hygiene)]

extern crate dynasm;
extern crate dynasmrt;

use dynasm::dynasm;
use dynasmrt::{DynasmApi, DynasmLabelApi};
use program::*;

use std::collections::HashMap;
use std::{env, mem, process};

mod program;

// macro_rules! binop {
//     ($op:instruction) => {{
//         dynasm!(asm
//             ; mov rax)
//     }}
// }

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
                    var_offsets.insert(dest.to_string(), 8 * num_vars);
                    num_vars += 1;
                }
            }
        }

        // 8 * #variables, rounded up to a multiple of 16
        let num_bytes = 16 * (num_vars / 2);

        let mut asm = dynasmrt::x64::Assembler::new().unwrap();

        let start = asm.offset();

        // prologue
        dynasm!(asm
            ; push rbp
            ; mov rbp, rsp
            ; sub rsp, num_bytes
        );

        for inst in &bril_func.instrs {
            match &inst.op {
                program::OpCode::BinOp(op) => {
                    if let (Some(args), Some(dest)) = (&inst.args, &inst.dest) {
                        if let (Some(&a), Some(&b), Some(&d)) = (
                            var_offsets.get(&args[0]),
                            var_offsets.get(&args[1]),
                            var_offsets.get(dest),
                        ) {
                            dynasm!(asm ; mov rax, [rbp - a]);
                            match op.as_ref() {
                                "add" => { dynasm!(asm ; add rax, [rbp - b]); }
                                "sub" => { dynasm!(asm ; sub rax, [rbp - b]); }
                                "mul" => { dynasm!(asm ; imul rax, [rbp - b]); }
                                "div" => { dynasm!(asm ; cqo ; idiv QWORD [rbp - b]); }
                                "eq" => { dynasm!(asm ; cmp rax, [rbp - b] ; sete al ; movzx rax, al); }
                                "lt" => { dynasm!(asm ; cmp rax, [rbp - b] ; setl al ; movzx rax, al); }
                                "gt" => { dynasm!(asm ; cmp rax, [rbp - b] ; setg al ; movzx rax, al); }
                                "le" => { dynasm!(asm ; cmp rax, [rbp - b] ; setle al ; movzx rax, al); }
                                "ge" => { dynasm!(asm ; cmp rax, [rbp - b] ; setge al ; movzx rax, al); }
                                _ => { }
                            }
                            dynasm!(asm ; mov [rbp - d], rax);
                        }
                    }
                }
                program::OpCode::Const => {
                    if let Some(dest) = &inst.dest {
                        match inst.value.as_ref().unwrap_or(&InstrType::VInt(0)) {
                            InstrType::VInt(value) => {
                                if let Some(&d) = var_offsets.get(dest) {
                                    dynasm!(asm
                                        ; mov rax, *value
                                        ; mov [rbp - d], rax
                                    );
                                }
                            }
                            InstrType::VBool(value) => panic!("Expected bool"),
                        }
                    }
                }
                program::OpCode::Print => {
                    if let Some(args) = &inst.args {
                        for arg in args {
                            if let Some(&a) = var_offsets.get(arg) {
                                dynasm!(asm
                                    ; mov rdi, [rbp - a]
                                    ; mov rax, QWORD print_int as _
                                    ; call rax
                                );
                            }
                        }
                    }
                }
                program::OpCode::Nop => {
                    dynasm!(asm ; nop);
                }
            }
        }

        // epilogue
        dynasm!(asm
            ; mov rsp, rbp
            ; pop rbp
            ; ret
        );

        let code = asm.finalize().unwrap();
        return AsmProgram {
            code: code,
            start: start,
        };
    }

    fn run(self) -> bool {
        let f: extern "stdcall" fn() -> bool = unsafe { mem::transmute(self.code.ptr(self.start)) };
        return f();
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Expected one argument");
        process::exit(1);
    }
    let bril_program = match program::read_json(&args[1]) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{}", e);
            eprintln!("Couldn't parse Bril file");
            process::exit(1);
        }
    };
    let asm_program = AsmProgram::compile(&bril_program.functions[0]);
    asm_program.run();
    // println!("{}", asm_program.run());
}

fn print_int(i: i64) {
    println!("{}", i);
}
