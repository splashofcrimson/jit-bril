extern crate dynasm;
extern crate dynasmrt;

use dynasm::dynasm;
use dynasmrt::{DynasmApi, DynasmLabelApi};
use super::program::InstrType::*;
use super::program::*;

use std::collections::HashMap;
use std::mem;

type Op = OpCode;

pub struct Env {
    env: HashMap<String, InstrType>,
}

impl Env {
    pub fn new() -> Env {
        Env {
            env: HashMap::<String, InstrType>::new(),
        }
    }

    pub fn get(&mut self, var_name: String) -> Option<InstrType> {
        self.env.get(&var_name).cloned()
    }

    pub fn put(&mut self, var_name: String, val: InstrType) {
        self.env.insert(var_name, val);
    }
}

pub enum Action {
    Next,
    Jump(String),
    Return,
}

struct AsmProgram {
    code: dynasmrt::ExecutableBuffer,
    start: dynasmrt::AssemblyOffset,
}

pub struct Interpreter {
    asm: dynasmrt::x64::Assembler,
    asm_map: HashMap::<i64, AsmProgram>,
    bril_map: HashMap::<i64, Function>,
    index_map: HashMap::<String, i64>,
    profile_map: HashMap::<i64, i64>,
    program: Program,
}

impl Interpreter {
    pub fn new(bril_ir: Program, jit: bool) -> Interpreter {
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
            program: bril_ir
        }
    }

    pub fn handle_call(&mut self, func_idx: i64) {
        if let Some(func_asm) = self.asm_map.get(&func_idx) {
            let func: fn(&Interpreter) = unsafe { mem::transmute(func_asm.code.ptr(func_asm.start)) };
            func(&self);
        } else {
            if let Some(func_profile_data) = self.profile_map.get(&func_idx) {
                if func_profile_data > &1 {
                    let func_bril = self.bril_map.remove(&func_idx).unwrap();
                    let func_asm = self.compile(&func_bril);
                    let func: fn(&Interpreter) = unsafe { mem::transmute(func_asm.code.ptr(func_asm.start)) };
                    self.asm_map.insert(func_idx, func_asm);
                    func(&self);
                }
                else {
                    let bril_func = self.bril_map.get(&func_idx).unwrap().clone();
                    let name = bril_func.name;
                    // let args = instr.args.unwrap().get(1..).unwrap().to_vec();
                    let new_env = &mut Env::new();
                    let mut called = false;
                    for func in &self.program.functions.clone() {
                        if func.name == name {
                            // match &func.args {
                            //     Some(params) => {
                            //         for i in 0..params.len() {
                            //             let name = params.get(i).unwrap().name.to_string();
                            //             let val = args.get(i).unwrap().to_string();
                            //             new_env.put(name, env.get(val).unwrap());
                            //             }
                            //         }
                            //     None => (),
                            // }
                            let result = self.eval_func(&func, new_env);
                            if !result {
                                println!("Failed when calling function");
                            }
                            called = true;
                        }
                    }
                }
            }
        }
    }
        // if called 
        //     match instr.dest {
        //         Some(dest) => {
        //             env.put(dest, new_env.get("_ rho".to_string()).unwrap());
        //         }

        //         None => (),
        //     }
        //     Ok(Action::Next)
        // } else {
        //     return Err("Function not found");
        // }

    pub fn eval_program(&mut self) {
        let env = &mut Env::new();
        for func in &self.program.functions.clone() {
            if func.name == "main" {
                self.eval_func(&func, env);
            }
        }
    }

    pub fn eval_func(&mut self, func: &Function, env: &mut Env) -> bool {
        let mut i = 0;
        while i < func.instrs.len() {
            let instr = func.instrs.get(i).unwrap().clone();
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
    
    pub fn compile(&mut self, bril_func: &Function) -> AsmProgram {
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
                        dynasm!(self.asm
                            ; mov rax, QWORD Interpreter::handle_call as _ 
                            ; mov rdi, [rbp - 8]
                            ; mov rsi, QWORD *self.index_map.get(&args[0]).unwrap()
                            ; call rax
                        );
                    }
                }
                Some(OpCode::Print) => {
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
                    // epilogue
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

     pub fn find_label(func: &Function, label: String) -> Option<usize> {
        let mut i = 0;
        while i < func.instrs.len() {
            if func.instrs.get(i).unwrap().label == Some(label.to_string()) {
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

    pub fn eval_instr(&mut self, instr: Instruction, env: &mut Env) -> Result<Action, &str> {
        match instr.op.unwrap_or(Op::Nop) {
            Op::Const => {
                env.put(instr.dest.unwrap(), instr.value.unwrap());
                Ok(Action::Next)
            }

            Op::Id => {
                let src = instr.args.unwrap().get(0).unwrap().to_string();
                let val = env.get(src).unwrap();
                env.put(instr.dest.unwrap(), val);
                Ok(Action::Next)
            }

            Op::BinOp(op) => {
                let v1 = instr.args.clone().unwrap().get(0).unwrap().to_string();
                let v2 = instr.args.clone().unwrap().get(1).unwrap().to_string();
                let val1 = match env.get(v1).unwrap() {
                    VInt(v) => v,
                    VBool(_) => return Err("Expected int, got bool")
                };
                let val2 = match env.get(v2).unwrap() {
                    VInt(v) => v,
                    VBool(_) => return Err("Expected int, got bool")
                };

                let dest = instr.dest.unwrap().to_string();
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
                    _ => return Err("Unknown binop")
                };
                Ok(Action::Next)
            }

            Op::BinOpBool(op) => {
                let v1 = instr.args.clone().unwrap().get(0).unwrap().to_string();
                let v2 = instr.args.clone().unwrap().get(1).unwrap().to_string();
                let val1 = match env.get(v1).unwrap() {
                    VInt(_) => return Err("Expected bool, got int"),
                    VBool(b) => b,
                };
                let val2 = match env.get(v2).unwrap() {
                    VInt(_) => return Err("Expected bool, got int"),
                    VBool(b) => b,
                };

                let dest = instr.dest.unwrap().to_string();
                match op.as_str() {
                    "and" => env.put(dest, VBool(val1 && val2)),
                    "or" => env.put(dest, VBool(val1 || val2)),
                    _ => return Err("Unknown boolean binop"),
                };
                Ok(Action::Next)
            }

            Op::UnOpBool(op) => {
                let v1 = instr.args.unwrap().get(0).unwrap().to_string();
                let val1 = match env.get(v1).unwrap() {
                    VInt(_) => return Err("Expected bool, got int"),
                    VBool(b) => b,
                };

                let dest = instr.dest.unwrap().to_string();
                match op.as_str() {
                    "not" => env.put(dest, VBool(!val1)),
                    _ => return Err("Unknown unop"),
                };
                Ok(Action::Next)
            }

            Op::Print => {
                for arg in instr.args.unwrap() {
                    match env.get(arg.to_string()).unwrap() {
                        VInt(v) => print!("{} ", v),
                        VBool(b) => print!("{} ", b),
                    };
                }
                println!();
                Ok(Action::Next)
            }

            Op::Jmp => {
                let label = instr.args.unwrap().get(0).unwrap().to_string();
                Ok(Action::Jump(label))
            }

            Op::Br => {
                let v1 = instr.args.clone().unwrap().get(0).unwrap().to_string();
                let val = match env.get(v1).unwrap() {
                    VInt(_) => return Err("Expected bool in br, got int"),
                    VBool(b) => b,
                };
                if val {
                    Ok(Action::Jump(instr.args.clone().unwrap().get(1).unwrap().to_string()))
                } else {
                    Ok(Action::Jump(instr.args.clone().unwrap().get(2).unwrap().to_string()))
                }
            }

            Op::Call => {
                let name = instr.args.clone().unwrap().get(0).unwrap().to_string();
                let func_idx = self.index_map.get(&name).unwrap().clone();
                self.handle_call(func_idx);
                Ok(Action::Next)
                // let args = instr.args.unwrap().get(1..).unwrap().to_vec();
                // let new_env = &mut Env::new();
                // let mut called = false;
                // for func in &self.program.functions {
                //     if func.name == name {
                //         match &func.args {
                //             Some(params) => {
                //                 for i in 0..params.len() {
                //                     let name = params.get(i).unwrap().name.to_string();
                //                     let val = args.get(i).unwrap().to_string();
                //                     new_env.put(name, env.get(val).unwrap());
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
                //     match instr.dest {
                //         Some(dest) => {
                //             env.put(dest, new_env.get("_ rho".to_string()).unwrap());
                //         }

                //         None => (),
                //     }
                //     Ok(Action::Next)
                // } else {
                //     return Err("Function not found");
                // }
            }

            Op::Ret => {
                if instr.args.clone().unwrap().len() > 0 {
                    let return_val = env.get(instr.args.unwrap().get(0).unwrap().to_string());
                    env.put("_ rho".to_string(), return_val.unwrap());
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