use super::program::InstrType::*;
use super::program::*;
use fnv::FnvHashMap;

extern crate dynasm;
extern crate dynasmrt;

use dynasm::dynasm;
use dynasmrt::{DynasmApi, DynasmLabelApi};

use std::collections::HashMap;
use std::mem;

static RETURN_VAR: &'static str = "_ rho";

type Op = OpCode;

pub struct AsmProgram {
    code: dynasmrt::ExecutableBuffer,
    start: dynasmrt::AssemblyOffset,
}

pub struct Env<'a> {
    env: FnvHashMap<&'a str, InstrType>,
}

impl<'a> Env<'a> {
    pub fn new() -> Env<'a> {
        Env {
            env: FnvHashMap::default(),
        }
    }

    pub fn get(&mut self, var_name: &'a str) -> Option<InstrType> {
        self.env.get(&var_name).cloned()
    }

    pub fn put(&mut self, var_name: &'a str, val: InstrType) {
        self.env.insert(var_name, val);
    }
}

pub enum Action<'a> {
    Next,
    Jump(&'a str),
    Return,
}

pub struct Interpreter<'a> {
    asm: dynasmrt::x64::Assembler,
    asm_map: HashMap<i64, AsmProgram>,
    bril_map: HashMap<i64, Function>,
    index_map: HashMap<String, i64>,
    profile_map: HashMap<i64, i64>,
    program: &'a Program,
}

impl<'a> Interpreter<'a> {
    pub fn new(bril_ir: &'a Program, jit: bool) -> Interpreter<'a> {
        let asm = dynasmrt::x64::Assembler::new().unwrap();
        let mut index_map = HashMap::<String, i64>::new();
        let mut bril_map = HashMap::<i64, Function>::new();
        let mut profile_map = HashMap::<i64, i64>::new();
        let asm_map = HashMap::<i64, AsmProgram>::new();

        let mut i = 0;
        for fun in &bril_ir.functions {
            bril_map.insert(i, fun.clone());
            index_map.insert(fun.name.clone(), i);
            profile_map.insert(i, i);
            i += 1;
        }

        Interpreter {
            asm: asm,
            asm_map: asm_map,
            bril_map: bril_map,
            index_map: index_map,
            profile_map: profile_map,
            program: bril_ir,
        }
    }

    pub fn handle_call(&mut self, func_idx: i64, args: Vec<i64>) -> Option<i64> {
        println!("Received args {:?}", args);
        if let Some(func_asm) = self.asm_map.get(&func_idx) {
            let func: fn(&Interpreter, Vec<i64>) -> Option<i64> = unsafe { mem::transmute(func_asm.code.ptr(func_asm.start)) };
            let x = func(&self, args);
            println!("pre-compiled func returned {:?}", x);
            return x;
        } else {
            if true {
                if true {
                    let func_bril = self.bril_map.remove(&func_idx).unwrap();
                    let func_asm = self.compile(&func_bril);
                    let func: fn(&Interpreter, Vec<i64>) -> Option<i64> = unsafe { mem::transmute(func_asm.code.ptr(func_asm.start)) };
                    self.asm_map.insert(func_idx, func_asm);
                    let x = func(&self, args);
                    println!("func returned {:?}", x);
                    return x;
                }
                else {
                    // TODO Run function in Bril!!
                }
            }
        }
        None
    }

    pub fn compile(&mut self, bril_func: &Function) -> AsmProgram {
        let mut var_offsets = HashMap::<String, i32>::new();
        let mut var_types = HashMap::<String, String>::new();
        let mut labels = HashMap::<String, dynasmrt::DynamicLabel>::new();
        let mut num_vars = 2;

        if let Some(args) = &bril_func.args {
            for arg in args {
                var_offsets.insert(arg.name.to_string(), 8 * num_vars);
                num_vars += 1;
            }
        }
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

        let num_args: i64 = if let Some(args) = &bril_func.args {
            args.len() as _
        } else {
            0
        };

        for inst in &bril_func.instrs {
            match &inst.op {
                Some(OpCode::BinOp(op)) => {
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
                Some(OpCode::BinOpBool(op)) => {
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
                Some(OpCode::UnOpBool(op)) => {
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
                Some(OpCode::Const) => {
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
                Some(OpCode::Call) => {
                    if let Some(args) = &inst.args {
                        let name = &args[0];
                        let num_args = &args.len() - 1;
                        let num_bytes = 16 * ((num_args + 1) / 2) as i32;
                        dynasm!(self.asm
                            ; sub rsp, num_bytes
                            ; mov rdi, rsp
                            ; mov rsi, num_args as i32
                            ; mov rcx, QWORD Vec::<i64>::with_capacity as _
                            ; call rcx
                        );
                        for i in 0..num_args {
                            let var = &args[i + 1];
                            let value = var_offsets.get(var).unwrap();
                            dynasm!(self.asm
                                ; mov rdi, rsp
                                ; mov rsi, [rbp - value]

                                ; mov rcx, QWORD Vec::<i64>::push as _
                                ; call rcx
                            );
                        }
                        dynasm!(self.asm
                            ; mov rax, QWORD Interpreter::handle_call as _ 
                            ; mov rdi, [rbp - 8]
                            ; mov rsi, QWORD *self.index_map.get(name).unwrap()
                            ; mov rdx, rsp
                            ; call rax
                        );
                        if let Some(dest) = &inst.dest {
                            let value = var_offsets.get(dest).unwrap();
                            dynasm!(self.asm
                                ; mov [rbp - value], rdx
                            );
                        }
                        dynasm!(self.asm
                            ; add rsp, num_bytes
                        );
                    }
                }
                Some(OpCode::Print) => {
                    if let Some(args) = &inst.args {
                        for arg in args {
                            if let Some(&a) = var_offsets.get(arg) {
                                dynasm!(self.asm
                                    ; mov rdi, [rbp - a]
                                    ; mov rax, QWORD print_int as _);
                                // if let Some(&inst_type) = var_types.get(arg).as_ref() {
                                //     match inst_type.as_ref() {
                                //         "int" => { dynasm!(self.asm ; mov rax, QWORD print_int as _); }
                                //         "bool" => { dynasm!(self.asm ; mov rax, QWORD print_bool as _); }
                                //         _ => { }
                                //     }
                                // }
                                dynasm!(self.asm ; call rax);
                            }
                        }
                        dynasm!(self.asm
                            ; mov rax, QWORD print_newline as _
                            ; call rax
                        );
                    }
                }
                Some(OpCode::Nop) => {
                    dynasm!(self.asm ; nop);
                }
                Some(OpCode::Jmp) => {
                    if let Some(args) = &inst.args {
                        let dyn_label = get_dyn_label(&mut self.asm, &mut labels, &args[0]);
                        dynasm!(self.asm ; jmp =>dyn_label);
                    }
                }
                Some(OpCode::Br) => {
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
                Some(OpCode::Ret) => {
                    if let Some(args) = &inst.args {
                        if !args.is_empty() {
                                let offset = var_offsets.get(&args[0]).unwrap();
                                dynasm!(self.asm
                                    ; mov rax, 1
                                    ; mov rdx, [rbp - offset]
                                );
                        } else {
                            dynasm!(self.asm
                                ; mov rax, 0
                                ; mov rdx, 0
                            );
                        }
                    }
                    dynasm!(self.asm
                        ; mov rsp, rbp
                        ; pop rbp
                        ; ret
                    );
                }
                Some(OpCode::Id) => {
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

    pub fn eval_program(&mut self) {
        self.handle_call(*self.index_map.get("main").unwrap(), Vec::new());
    }

    pub fn find_label(func: &Function, label: &str) -> Option<usize> {
        let mut i = 0;
        let some_label = Some(label.to_string());
        while i < func.instrs.len() {
            if func.instrs.get(i).unwrap().label == some_label {
                break;
            }
            i += 1;
        }
        if i < func.instrs.len() {
            Some(i)
        } else {
            None
        }
    }

    pub fn eval_func(&mut self, func: &'a Function, env: &mut Env<'a>) -> bool {
        let mut i = 0;
        while i < func.instrs.len() {
            let instr = &func.instrs[i];
            let action = self.eval_instr(instr, env);
            match action {
                Ok(Action::Next) => {
                    i += 1;
                }
                Ok(Action::Jump(label)) => match Interpreter::find_label(&func, label) {
                    Some(v) => i = v,
                    None => {
                        println!("Couldn't find label to jump to");
                        return false;
                    }
                },
                Ok(Action::Return) => break,
                Err(s) => {
                    println!("{}", s);
                    return false;
                }
            }
        }
        true
    }

    pub fn eval_instr(&mut self, instr: &'a Instruction, env: &mut Env<'a>) -> Result<Action, &str> {
        match instr.op.as_ref().unwrap_or(&Op::Nop) {
            Op::Const => {
                env.put(
                    instr.dest.as_ref().unwrap(),
                    instr.value.as_ref().unwrap().to_owned(),
                );
                Ok(Action::Next)
            }

            Op::Id => {
                let instr_args = &(instr.args).as_ref().unwrap();
                let src = &instr_args[0];
                let val = env.get(src).unwrap();
                env.put(instr.dest.as_ref().unwrap(), val);
                Ok(Action::Next)
            }

            Op::BinOp(op) => {
                let instr_args = &(instr.args).as_ref().unwrap();
                let v1 = &instr_args[0];
                let v2 = &instr_args[1];
                let val1 = match env.get(v1).unwrap() {
                    VInt(v) => v,
                    VBool(_) => return Err("Expected int, got bool"),
                };
                let val2 = match env.get(v2).unwrap() {
                    VInt(v) => v,
                    VBool(_) => return Err("Expected int, got bool"),
                };

                let dest = instr.dest.as_ref().unwrap();
                match op.as_str() {
                    "add" => env.put(dest, VInt(val1 + val2)),
                    "mul" => env.put(dest, VInt(val1 * val2)),
                    "sub" => env.put(dest, VInt(val1 - val2)),
                    "div" => env.put(dest, VInt(val1 / val2)),
                    "le" => env.put(dest, VBool(val1 <= val2)),
                    "lt" => env.put(dest, VBool(val1 < val2)),
                    "gt" => env.put(dest, VBool(val1 > val2)),
                    "ge" => env.put(dest, VBool(val1 >= val2)),
                    "eq" => env.put(dest, VBool(val1 == val2)),
                    _ => return Err("Unknown binop"),
                };
                Ok(Action::Next)
            }

            Op::BinOpBool(op) => {
                let instr_args = &(instr.args).as_ref().unwrap();
                let v1 = &instr_args[0];
                let v2 = &instr_args[1];
                let val1 = match env.get(v1).unwrap() {
                    VInt(_) => return Err("Expected bool, got int"),
                    VBool(b) => b,
                };
                let val2 = match env.get(v2).unwrap() {
                    VInt(_) => return Err("Expected bool, got int"),
                    VBool(b) => b,
                };

                let dest = instr.dest.as_ref().unwrap();
                match op.as_str() {
                    "and" => env.put(dest, VBool(val1 && val2)),
                    "or" => env.put(dest, VBool(val1 || val2)),
                    _ => return Err("Unknown boolean binop"),
                };
                Ok(Action::Next)
            }

            Op::UnOpBool(op) => {
                let instr_args = &(instr.args).as_ref().unwrap();
                let v1 = &instr_args[0];
                let val1 = match env.get(v1).unwrap() {
                    VInt(_) => return Err("Expected bool, got int"),
                    VBool(b) => b,
                };

                let dest = instr.dest.as_ref().unwrap();
                match op.as_str() {
                    "not" => env.put(dest, VBool(!val1)),
                    _ => return Err("Unknown unop"),
                };
                Ok(Action::Next)
            }

            Op::Print => {
                let instr_args = &(instr.args).as_ref().unwrap();
                for arg in *instr_args {
                    match env.get(&arg).unwrap() {
                        VInt(v) => print!("{} ", v),
                        VBool(b) => print!("{} ", b),
                    };
                }
                println!();
                Ok(Action::Next)
            }

            Op::Jmp => {
                let label = &instr.args.as_ref().unwrap()[0];
                Ok(Action::Jump(&label))
            }

            Op::Br => {
                let instr_args = &(instr.args).as_ref().unwrap();
                let v1 = &instr_args[0];
                let val = match env.get(v1).unwrap() {
                    VInt(_) => return Err("Expected bool in br, got int"),
                    VBool(b) => b,
                };
                if val {
                    Ok(Action::Jump(&instr_args[1]))
                } else {
                    Ok(Action::Jump(&instr_args[2]))
                }
            }

            Op::Call => {
                let instr_args = &(instr.args).as_ref().unwrap();
                let name = &instr_args[0];
                // let new_env = &mut Env::new();
                // let mut called = false;
                // // TODO handle call
                // for func in &self.program.functions {
                //     if func.name == *name {
                //         match &func.args {
                //             Some(params) => {
                //                 for i in 0..params.len() {
                //                     let name = &params.get(i).unwrap().name;
                //                     let val = &instr_args[i + 1];
                //                     new_env.put(&name, env.get(&val).unwrap());
                //                 }
                //             }

                //             None => (),
                //         }
                //         let result = self.eval_func(&func, new_env);
                //         if !result {
                //             return Err("Failed when calling function");
                //         }
                //         called = true;
                //     }
                // }
                // if called {
                //     match instr.dest.as_ref() {
                //         Some(dest) => {
                //             env.put(dest, new_env.get("_ rho").unwrap());
                //         }

                //         None => (),
                //     }
                //     Ok(Action::Next)
                // } else {
                //     return Err("Function not found");
                // }
                let func_idx = self.index_map.get(name).unwrap().clone();
                let mut args = Vec::new();
                for arg in &instr_args[1..] {
                    let a = match env.get(&arg).unwrap() {
                        VInt(v) => v,
                        VBool(b) => b as i64
                    };
                    args.push(a);
                }
                self.handle_call(func_idx, Vec::new());
                Ok(Action::Next)
            }

            Op::Ret => {
                let instr_args = &(instr.args).as_ref().unwrap();
                if instr_args.len() > 0 {
                    let return_val = env.get(&instr_args[0]);
                    env.put(&RETURN_VAR, return_val.unwrap());
                }
                Ok(Action::Return)
            }

            Op::Nop => Ok(Action::Next),
        }
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
