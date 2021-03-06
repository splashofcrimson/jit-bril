use super::program::*;
use fnv::FnvHashMap;

static RETURN_VAR: &'static str = "_ rho";

type Op = OpCode;

pub struct Env<'a> {
    env: FnvHashMap<&'a str, i64>,
}

impl<'a> Env<'a> {
    pub fn new() -> Env<'a> {
        Env {
            env: FnvHashMap::default(),
        }
    }

    pub fn get(&mut self, var_name: &'a str) -> Option<i64> {
        self.env.get(&var_name).cloned()
    }

    pub fn put(&mut self, var_name: &'a str, val: i64) {
        self.env.insert(var_name, val);
    }
}

pub enum Action<'a> {
    Next,
    Jump(&'a str),
    Return,
}

pub struct Interpreter<'a> {
    program: &'a Program,
}

impl<'a> Interpreter<'a> {
    pub fn new(bril_ir: &'a Program) -> Interpreter<'a> {
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

    pub fn eval_func(&self, func: &'a Function, env: &mut Env<'a>) -> bool {
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

    pub fn eval_instr(&self, instr: &'a Instruction, env: &mut Env<'a>) -> Result<Action, &str> {
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
                let val1 = env.get(v1).unwrap();
                let val2 = env.get(v2).unwrap();

                let dest = instr.dest.as_ref().unwrap();
                match op.as_str() {
                    "add" => env.put(dest, val1 + val2),
                    "mul" => env.put(dest, val1 * val2),
                    "sub" => env.put(dest, val1 - val2),
                    "div" => env.put(dest, val1 / val2),
                    "le" => env.put(dest, (val1 <= val2) as i64),
                    "lt" => env.put(dest, (val1 < val2) as i64),
                    "gt" => env.put(dest, (val1 > val2) as i64),
                    "ge" => env.put(dest, (val1 >= val2) as i64),
                    "eq" => env.put(dest, (val1 == val2) as i64),
                    _ => return Err("Unknown binop"),
                };
                Ok(Action::Next)
            }

            Op::BinOpBool(op) => {
                let instr_args = &(instr.args).as_ref().unwrap();
                let v1 = &instr_args[0];
                let v2 = &instr_args[1];
                let val1 = env.get(v1).unwrap() != 0;
                let val2 = env.get(v2).unwrap() != 0;

                let dest = instr.dest.as_ref().unwrap();
                match op.as_str() {
                    "and" => env.put(dest, (val1 && val2) as i64),
                    "or" => env.put(dest, (val1 || val2) as i64),
                    _ => return Err("Unknown boolean binop"),
                };
                Ok(Action::Next)
            }

            Op::UnOpBool(op) => {
                let instr_args = &(instr.args).as_ref().unwrap();
                let v1 = &instr_args[0];
                let val1 = env.get(v1).unwrap() != 0;

                let dest = instr.dest.as_ref().unwrap();
                match op.as_str() {
                    "not" => env.put(dest, (!val1) as i64),
                    _ => return Err("Unknown unop"),
                };
                Ok(Action::Next)
            }

            Op::Print => {
                let instr_args = &(instr.args).as_ref().unwrap();
                for arg in *instr_args {
                    print!("{} ", env.get(&arg).unwrap());
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
                let val = env.get(v1).unwrap() != 0;
                if val {
                    Ok(Action::Jump(&instr_args[1]))
                } else {
                    Ok(Action::Jump(&instr_args[2]))
                }
            }

            Op::Call => {
                let instr_args = &(instr.args).as_ref().unwrap();
                let name = &instr_args[0];
                let new_env = &mut Env::new();
                let mut called = false;
                for func in &self.program.functions {
                    if func.name == *name {
                        match &func.args {
                            Some(params) => {
                                for i in 0..params.len() {
                                    let name = &params.get(i).unwrap().name;
                                    let val = &instr_args[i + 1];
                                    new_env.put(&name, env.get(&val).unwrap());
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
                            env.put(dest, new_env.get("_ rho").unwrap());
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
                    let return_val = env.get(&instr_args[0]);
                    env.put(&RETURN_VAR, return_val.unwrap());
                }
                Ok(Action::Return)
            }

            Op::Nop => Ok(Action::Next),
        }
    }
}
