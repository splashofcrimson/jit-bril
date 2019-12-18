#![feature(proc_macro_hygiene)]

extern crate dynasm;
extern crate dynasmrt;

use dynasm::dynasm;
use dynasmrt::{DynasmApi, DynasmLabelApi};
use program::*;

use std::collections::HashMap;
use std::{env, mem, process};

mod program;

struct BrilProgram {
    asm: dynasmrt::x64::Assembler,
    bril_ir: program::Program,
    compilation_map: HashMap::<String, AsmProgram>,
}

struct AsmProgram {
    code: dynasmrt::ExecutableBuffer,
    start: dynasmrt::AssemblyOffset,
}

impl BrilProgram {
    pub fn new(bril_program: program::Program) -> BrilProgram {
        let asm = dynasmrt::x64::Assembler::new().unwrap();
        let compilation_map = HashMap::<String, AsmProgram>::new();

        let mut bril = BrilProgram {
            asm: asm, 
            bril_ir: bril_program.clone(),
            compilation_map: compilation_map,
        };
        let bril_func = bril.find_func("main").unwrap();
        let main_asm = bril.compile(&bril_func);
        bril.compilation_map.insert("main".to_string(), main_asm);

        bril
    }

    pub fn run(self) -> bool {
        let main_asm = self.compilation_map.get("main").unwrap();
        let f: extern "stdcall" fn() -> bool = unsafe { mem::transmute(main_asm.code.ptr(main_asm.start)) };
        return f();
    }

    fn call_func(&mut self, func_name: &str) {
        let func = self.find_func(&func_name).unwrap();
        let func_asm = self.compile(&func);
        let f: extern "stdcall" fn() -> bool = unsafe { mem::transmute(func_asm.code.ptr(func_asm.start)) };
        f();
    }
    
    fn find_func(&mut self, func_name: &str) -> Option<program::Function> {
        for func in self.bril_ir.functions.clone() {
            if func.name == func_name {
                return Some(func.clone());
            }
        }

        None
    }

    // fn func_call(&mut self, fun: String) {
    //     dynasm!(asm
    //         ; )
    // }

    pub fn compile(&mut self, bril_func: &program::Function) -> AsmProgram {
        let mut var_offsets = HashMap::<String, i32>::new();
        let mut var_types = HashMap::<String, String>::new();
        let mut labels = HashMap::<String, dynasmrt::DynamicLabel>::new();
        let mut num_vars = 1;

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
                        match inst.value.as_ref().unwrap_or(&InstrType::VInt(0)) {
                            InstrType::VInt(value) => {
                                if let Some(&d) = var_offsets.get(dest) {
                                    dynasm!(self.asm
                                        ; mov rax, *value
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
                    dynasm!(self.asm ;; self.call_func("sixnine" as _));
                }
                // program::OpCode::Call => {
                //     if let Some(args) = &inst.args {
                //         dynasm!(asm
                //             ; ->sixnine:
                //             ; .bytes string.as_bytes()
                //             ; lea rcx, [->sixnine]
                //             ; xor edx, edx
                //             ; mov dl, BYTE string.len() as _
                //             ; mov rax, QWORD call_func as _
                //         );
                //         );
                //     }
                // }
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

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Expected one argument");
        process::exit(1);
    }
    let bril_ir = match program::read_json(&args[1]) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{}", e);
            eprintln!("Couldn't parse Bril file");
            process::exit(1);
        }
    };
    let bril_program = BrilProgram::new(bril_ir);
    bril_program.run();
    // println!("{}", asm_program.run());
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
fn get_dyn_label(asm: &mut dynasmrt::x64::Assembler, labels: &mut HashMap::<String, dynasmrt::DynamicLabel>, label: &str) ->
        dynasmrt::DynamicLabel {
    if let Some(&dyn_label) = labels.get(label) {
        return dyn_label;
    } else {
        let dyn_label = asm.new_dynamic_label();
        labels.insert(label.to_string(), dyn_label);
        return dyn_label;
    }
}

// fn call_func(program: &program, name: String) {
//     let callee = AsmProgram::compile(&program, name.as_ref());
//     callee.run();
// }
