use std::collections::HashMap;

use super::program::InstrType::*;
use super::program::*;

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

pub struct Interpreter {
    program: Program,
}

impl Interpreter {
    pub fn new(bril_ir: Program) -> Interpreter {
        Interpreter { program: bril_ir }
    }

    pub fn eval_program(&self) {
        let env = &mut Env::new();
        for func in &self.program.functions {
            if func.name == "main" {
                self.eval_func(&func, env);
            }
        }
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

    pub fn eval_func(&self, func: &Function, env: &mut Env) -> bool {
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

    pub fn eval_instr(&self, instr: &Instruction, env: &mut Env) -> Result<Action, &str> {
        match instr.op.as_ref().unwrap_or(&Op::Nop) {
            Op::Const => {
                env.put(
                    instr.dest.as_ref().unwrap().to_string(),
                    instr.value.as_ref().unwrap().clone(),
                );
                Ok(Action::Next)
            }

            Op::Id => {
                let instr_args = &(instr.args).as_ref().unwrap();
                let src = instr_args[0].to_string();
                let val = env.get(src).unwrap();
                env.put(instr.dest.as_ref().unwrap().to_string(), val);
                Ok(Action::Next)
            }

            Op::BinOp(op) => {
                let instr_args = &(instr.args).as_ref().unwrap();
                let v1 = instr_args[0].to_string();
                let v2 = instr_args[1].to_string();
                let val1 = match env.get(v1).unwrap() {
                    VInt(v) => v,
                    VBool(_) => return Err("Expected int, got bool"),
                };
                let val2 = match env.get(v2).unwrap() {
                    VInt(v) => v,
                    VBool(_) => return Err("Expected int, got bool"),
                };

                let dest = instr.dest.as_ref().unwrap().to_string();
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
                let v1 = instr_args[0].to_string();
                let v2 = instr_args[1].to_string();
                let val1 = match env.get(v1).unwrap() {
                    VInt(_) => return Err("Expected bool, got int"),
                    VBool(b) => b,
                };
                let val2 = match env.get(v2).unwrap() {
                    VInt(_) => return Err("Expected bool, got int"),
                    VBool(b) => b,
                };

                let dest = instr.dest.as_ref().unwrap().to_string();
                match op.as_str() {
                    "and" => env.put(dest, VBool(val1 && val2)),
                    "or" => env.put(dest, VBool(val1 || val2)),
                    _ => return Err("Unknown boolean binop"),
                };
                Ok(Action::Next)
            }

            Op::UnOpBool(op) => {
                let instr_args = &(instr.args).as_ref().unwrap();
                let v1 = instr_args[0].to_string();
                let val1 = match env.get(v1).unwrap() {
                    VInt(_) => return Err("Expected bool, got int"),
                    VBool(b) => b,
                };

                let dest = instr.dest.as_ref().unwrap().to_string();
                match op.as_str() {
                    "not" => env.put(dest, VBool(!val1)),
                    _ => return Err("Unknown unop"),
                };
                Ok(Action::Next)
            }

            Op::Print => {
                let instr_args = &(instr.args).as_ref().unwrap();
                for arg in *instr_args {
                    match env.get(arg.to_string()).unwrap() {
                        VInt(v) => print!("{} ", v),
                        VBool(b) => print!("{} ", b),
                    };
                }
                println!();
                Ok(Action::Next)
            }

            Op::Jmp => {
                let label = instr.args.as_ref().unwrap()[0].to_string();
                Ok(Action::Jump(label))
            }

            Op::Br => {
                let instr_args = &(instr.args).as_ref().unwrap();
                let v1 = instr_args[0].to_string();
                let val = match env.get(v1).unwrap() {
                    VInt(_) => return Err("Expected bool in br, got int"),
                    VBool(b) => b,
                };
                if val {
                    Ok(Action::Jump(instr_args[1].to_string()))
                } else {
                    Ok(Action::Jump(instr_args[2].to_string()))
                }
            }

            Op::Call => {
                let instr_args = &(instr.args).as_ref().unwrap();
                let name = &instr_args[0];
                let new_env = &mut Env::new();
                let mut called = false;
                for func in &self.program.functions {
                    if func.name == name.to_string() {
                        match &func.args {
                            Some(params) => {
                                for i in 0..params.len() {
                                    let name = params.get(i).unwrap().name.to_string();
                                    let val = &instr_args[i + 1];
                                    new_env.put(name, env.get(val.to_string()).unwrap());
                                }
                            }

                            None => (),
                        }
                        let result = self.eval_func(&func, new_env);
                        if !result {
                            return Err("Failed when calling function");
                        }
                        called = true;
                    }
                }
                if called {
                    match instr.dest.as_ref() {
                        Some(dest) => {
                            env.put(dest.to_string(), new_env.get("_ rho".to_string()).unwrap());
                        }

                        None => (),
                    }
                    Ok(Action::Next)
                } else {
                    return Err("Function not found");
                }
            }

            Op::Ret => {
                let instr_args = &(instr.args).as_ref().unwrap();
                if instr_args.len() > 0 {
                    let return_val = env.get(instr_args[0].to_string());
                    env.put("_ rho".to_string(), return_val.unwrap());
                }
                Ok(Action::Return)
            }

            Op::Nop => Ok(Action::Next),
        }
    }
}
