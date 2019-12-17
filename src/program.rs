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
    BinOpBool(String),
    Const,
    Nop,
    Print,
    UnOpBool(String),
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
      "and" | "or" => OpCode::BinOpBool(op),
      "not" => OpCode::UnOpBool(op),
      "const" => OpCode::Const,
      "print" => OpCode::Print,
      _ => OpCode::Nop
    }
  }
}

pub fn read_json(file_name: &str) -> Result<Program, Box<dyn Error>> {
    let prog_file = File::open(file_name)?;
    let prog_reader = BufReader::new(prog_file);
    let prog_json = serde_json::from_reader(prog_reader)?;

    Ok(prog_json)
}
