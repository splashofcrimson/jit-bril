use serde::{Deserialize, Serialize};

use std::error::Error;
use std::fs::File;
use std::io::BufReader;

#[derive(Clone, Deserialize, Debug, Serialize)]
#[serde(untagged)]
pub enum InstrType {
    VInt(i32),
    VBool(bool),
}

#[derive(Clone, Deserialize, Debug, Serialize)]
#[serde(from = "String")]
pub enum OpCode {
    BinOp(String),
    Const,
    Nop,
    Print,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct Program {
    pub functions: Vec<Function>,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct Function {
    pub instrs: Vec<Instruction>,
    name: String,
}

#[derive(Clone, Deserialize, Debug, Serialize)]
pub struct Instruction {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dest: Option<String>,
    pub op: OpCode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<InstrType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
}

impl From<String> for OpCode {
  fn from(op: String) -> Self {
    match op.as_ref() {
      "nop" => OpCode::Nop,
      "add" | "mul" | "sub" | "div" | "eq" | "lt" | "gt" | "le" | "ge" => OpCode::BinOp(op),
      "const" => OpCode::Const,
      "print" => OpCode::Print,
      _ => OpCode::Nop
    }
  }
}

// impl Program {
//     pub fn new(function: Function) -> Program {
//       let program = Program {
//         functions: vec![function],
//       };
//       return program;
//     }
// }

// impl Function {
//     pub fn new(instrs: Vec<Instruction>) -> Function {
//       let function = Function {
//         instrs: instrs,
//         name: "main".to_string(),
//       };
//       return function;
//     }
// }

// impl Instruction {
//     pub fn new(args: Option<Vec<String>>, dest: Option<String>, op: String, value: Option<i32>, vtype: Option<String>) -> Option<Instruction> {
//       let instruction = match op.as_ref() {
//         "nop" => Some(Instruction {
//           args: Some(Vec::new()),
//           dest: None,
//           op: "nop".to_string(),
//           value: None,
//           r#type: None,
//         }),
//         "add" | "mul" | "sub" | "div" => Some(Instruction {
//           args: args,
//           dest: dest,
//           op: op,
//           value: None,
//           r#type: Some("int".to_string()),
//         }),
//         "lt" | "le" | "gt" | "ge" | "eq" | "and" | "or" | "not" => Some(Instruction {
//           args: args,
//           dest: dest,
//           op: op,
//           value: None,
//           r#type: Some("bool".to_string()),
//         }),
//         "const" => Some(Instruction {
//           args: None,
//           dest: dest,
//           op: op,
//           value: value,
//           r#type: vtype,
//         }),
//         "id" => Some(Instruction {
//           args: args,
//           dest: dest,
//           op: op,
//           value: None,
//           r#type: None,
//         }),
//         _ => None
//       }
//       return instruction;
//     }

// }

pub fn read_json(file_name: &str) -> Result<Program, Box<dyn Error>> {
    let prog_file = File::open(file_name)?;
    let prog_reader = BufReader::new(prog_file);
    let prog_json = serde_json::from_reader(prog_reader)?;

    Ok(prog_json)
}
