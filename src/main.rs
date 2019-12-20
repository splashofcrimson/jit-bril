#![feature(proc_macro_hygiene)]

extern crate dynasm;
extern crate dynasmrt;

use dynasm::dynasm;
use dynasmrt::{DynasmApi, DynasmLabelApi};
use program::*;
use interpreter::Interpreter;

use std::collections::HashMap;
use std::{env, mem, process, io::{self, Read}};

mod program;
mod interpreter;

struct BrilProgram {
    asm: dynasmrt::x64::Assembler,
    index_map: HashMap::<String, i64>,
    asm_map: HashMap::<i64, AsmProgram>,
    bril_map: HashMap::<i64, program::Function>,
}

struct AsmProgram {
    code: dynasmrt::ExecutableBuffer,
    start: dynasmrt::AssemblyOffset,
}

impl BrilProgram {
    pub fn new(bril_program: program::Program) -> BrilProgram {
        let asm = dynasmrt::x64::Assembler::new().unwrap();
        let mut index_map = HashMap::<String, i64>::new();
        let mut bril_map = HashMap::<i64, program::Function>::new();
        let asm_map = HashMap::<i64, AsmProgram>::new();

        let mut i = 0;
        for fun in &bril_program.functions {
            bril_map.insert(i, fun.clone());
            index_map.insert(fun.name.clone(), i);
            i += 1;
        }

        return BrilProgram {
            asm: asm, 
            index_map: index_map,
            asm_map: asm_map,
            bril_map: bril_map
        }
    }

    fn compile_and_run(&mut self, func_idx: i64) {
        if let Some(func_asm) = self.asm_map.get(&func_idx) {
            let func: fn(&BrilProgram) = unsafe { mem::transmute(func_asm.code.ptr(func_asm.start)) };
            func(&self);
        } else {
            let func_bril = self.bril_map.remove(&func_idx).unwrap();
            let func_asm = self.compile(&func_bril);
            let func: fn(&BrilProgram) = unsafe { mem::transmute(func_asm.code.ptr(func_asm.start)) };
            self.asm_map.insert(func_idx, func_asm);
            func(&self)
        }
    }

    pub fn compile(&mut self, bril_func: &program::Function) -> AsmProgram {
        let mut var_offsets = HashMap::<String, i32>::new();
        let mut var_types = HashMap::<String, String>::new();
        let mut labels = HashMap::<String, dynasmrt::DynamicLabel>::new();
        let mut num_vars = 2;

        for inst in &bril_func.instrs {
            if let Some(dest) = &inst.dest {
                if !var_offsets.contains_key(dest) {
                    var_offsets.insert(dest.to_string(), 8 * num_vars);
                    num_vars += 1;
                }
                if !var_types.contains_key(dest) {
                    if let Some(inst_type) = &inst.r#type {
                        var_types.insert(dest.to_string(), inst_type.to_string());
                    }
                }
            }
        }

        // 8 * #variables, rounded up to a multiple of 16
        let num_bytes = 16 * (num_vars / 2);

        let start = self.asm.offset();

        // prologue
        dynasm!(self.asm
            ; push rbp
            ; mov rbp, rsp
            ; sub rsp, num_bytes
            ; mov [rbp - 8], rdi
        );

        for inst in &bril_func.instrs {
            match &inst.op {
                Some(program::OpCode::BinOp(op)) => {
                    if let (Some(args), Some(dest)) = (&inst.args, &inst.dest) {
                        if let (Some(&a), Some(&b), Some(&d)) = (
                            var_offsets.get(&args[0]),
                            var_offsets.get(&args[1]),
                            var_offsets.get(dest),
                        ) {
                            dynasm!(self.asm ; mov rax, [rbp - a]);
                            match op.as_ref() {
                                "add" => { dynasm!(self.asm ; add rax, [rbp - b]); }
                                "sub" => { dynasm!(self.asm ; sub rax, [rbp - b]); }
                                "mul" => { dynasm!(self.asm ; imul rax, [rbp - b]); }
                                "div" => { dynasm!(self.asm ; cqo ; idiv QWORD [rbp - b]); }
                                "eq" => { dynasm!(self.asm ; cmp rax, [rbp - b] ; sete al ; movzx rax, al); }
                                "lt" => { dynasm!(self.asm ; cmp rax, [rbp - b] ; setl al ; movzx rax, al); }
                                "gt" => { dynasm!(self.asm ; cmp rax, [rbp - b] ; setg al ; movzx rax, al); }
                                "le" => { dynasm!(self.asm ; cmp rax, [rbp - b] ; setle al ; movzx rax, al); }
                                "ge" => { dynasm!(self.asm ; cmp rax, [rbp - b] ; setge al ; movzx rax, al); }
                                _ => { }
                            }
                            dynasm!(self.asm ; mov [rbp - d], rax);
                        }
                    }
                }
                Some(program::OpCode::BinOpBool(op)) => {
                    if let (Some(args), Some(dest)) = (&inst.args, &inst.dest) {
                        if let (Some(&a), Some(&b), Some(&d)) = (
                            var_offsets.get(&args[0]),
                            var_offsets.get(&args[1]),
                            var_offsets.get(dest),
                         ) {
                            dynasm!(self.asm ; mov rax, [rbp - a]);
                            match op.as_ref() {
                                "and" => { dynasm!(self.asm ; and rax, [rbp - b]); }
                                "or" => { dynasm!(self.asm ; or rax, [rbp - b]); }
                                _ => { }
                            }
                            dynasm!(self.asm ; mov [rbp - d], rax);
                        }
                    }
                }
                Some(program::OpCode::UnOpBool(op)) => {
                    if let (Some(args), Some(dest)) = (&inst.args, &inst.dest) {
                        if let (Some(&a), Some(&d)) = (
                            var_offsets.get(&args[0]),
                            var_offsets.get(dest),
                        ) {
                            dynasm!(self.asm ; mov rax, [rbp - a]);
                            match op.as_ref() {
                                "not" => { dynasm!(self.asm ; xor rax, 1); }
                                _ => { }
                            }
                            dynasm!(self.asm ; mov [rbp - d], rax);
                        }
                    }
                }
                Some(program::OpCode::Const) => {
                    if let Some(dest) = &inst.dest {
                        match inst.value.as_ref().unwrap() {
                            InstrType::VInt(value) => {
                                if let Some(&d) = var_offsets.get(dest) {
                                    dynasm!(self.asm
                                        ; mov rax, QWORD *value
                                        ; mov [rbp - d], rax
                                    );
                                }
                            }
                            InstrType::VBool(value) => {
                                let value_int = *value as i32;
                                if let Some(&d) = var_offsets.get(dest) {
                                    dynasm!(self.asm
                                        ; mov rax, value_int
                                        ; mov [rbp - d], rax
                                    );
                                }
                            },
                        }
                    }
                }
                Some(program::OpCode::Call) => {
                    if let Some(args) = &inst.args {
                        dynasm!(self.asm
                            ; mov rax, QWORD BrilProgram::compile_and_run as _ 
                            ; mov rdi, [rbp - 8]
                            ; mov rsi, QWORD *self.index_map.get(&args[0]).unwrap()
                            ; call rax
                        );
                    }
                }
                Some(program::OpCode::Print) => {
                    if let Some(args) = &inst.args {
                        for arg in args {
                            if let Some(&a) = var_offsets.get(arg) {
                                dynasm!(self.asm ; mov rdi, [rbp - a]);
                                if let Some(&inst_type) = var_types.get(arg).as_ref() {
                                    match inst_type.as_ref() {
                                        "int" => { dynasm!(self.asm ; mov rax, QWORD print_int as _); }
                                        "bool" => { dynasm!(self.asm ; mov rax, QWORD print_bool as _); }
                                        _ => { }
                                    }
                                }
                                dynasm!(self.asm ; call rax);
                            }
                        }
                        dynasm!(self.asm
                            ; mov rax, QWORD print_newline as _
                            ; call rax
                        );
                    }
                }
                Some(program::OpCode::Nop) => {
                    dynasm!(self.asm ; nop);
                }
                Some(program::OpCode::Jmp) => {
                    if let Some(args) = &inst.args {
                        let dyn_label = get_dyn_label(&mut self.asm, &mut labels, &args[0]);
                        dynasm!(self.asm ; jmp =>dyn_label);
                    }
                }
                Some(program::OpCode::Br) => {
                    if let Some(args) = &inst.args {
                        if let Some(&b) = var_offsets.get(&args[0]) {
                            let dyn_label_true = get_dyn_label(&mut self.asm, &mut labels, &args[1]);
                            let dyn_label_false = get_dyn_label(&mut self.asm, &mut labels, &args[2]);
                            dynasm!(self.asm
                                ; test [rbp - b], 1
                                ; jne =>dyn_label_true
                                ; jmp =>dyn_label_false
                            );
                        }
                    }

                }
                Some(program::OpCode::Ret) => {
                    // epilogue
                    dynasm!(self.asm
                        ; mov rsp, rbp
                        ; pop rbp
                        ; ret
                    );
                }
                Some(program::OpCode::Id) => {
                    if let (Some(args), Some(dest)) = (&inst.args, &inst.dest) {
                        if let (Some(&a), Some(&d)) = (
                            var_offsets.get(&args[0]),
                            var_offsets.get(dest),
                        ) {
                            dynasm!(self.asm ; mov rax, [rbp - a]);
                            dynasm!(self.asm ; mov [rbp - d], rax);
                        }
                    }
                }
                None => {
                    if let Some(label) = &inst.label {
                        let dyn_label = get_dyn_label(&mut self.asm, &mut labels, label);
                        dynasm!(self.asm ; =>dyn_label);
                    }
                }
            }
        }

        // epilogue
        dynasm!(self.asm
            ; mov rsp, rbp
            ; pop rbp
            ; ret
        );
        let mut asm_final = dynasmrt::x64::Assembler::new().unwrap();
        mem::swap(&mut self.asm, &mut asm_final);
        let code = asm_final.finalize().unwrap();
        return AsmProgram {
            code: code,
            start: start,
        };
    }
}

fn print_int(i: i64) {
    print!("{} ", i);
}

fn print_bool(b: bool) {
    print!("{} ", b);
}

fn print_newline() {
    println!()
}

fn get_dyn_label(
        asm: &mut dynasmrt::x64::Assembler,
        labels: &mut HashMap::<String, dynasmrt::DynamicLabel>,
        label: &str) -> dynasmrt::DynamicLabel {
    if let Some(&dyn_label) = labels.get(label) {
        return dyn_label;
    } else {
        let dyn_label = asm.new_dynamic_label();
        labels.insert(label.to_string(), dyn_label);
        return dyn_label;
    }
}

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
        let mut bril_program = BrilProgram::new(bril_ir);
        let main_idx: i64 = *bril_program.index_map.get("main").unwrap();
        bril_program.compile_and_run(main_idx);
    }
    else {
        eprintln!("Invalid mode");
    }

}

