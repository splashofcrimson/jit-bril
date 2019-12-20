use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Debug, Serialize)]
#[serde(untagged)]
pub enum InstrType {
    VInt(i64),
    VBool(bool),
}

#[derive(Clone, Deserialize, Debug, Serialize)]
#[serde(from = "String")]
pub enum OpCode {
    BinOp(String),
    BinOpBool(String),
    Call,
    Const,
    Nop,
    Print,
    Jmp,
    Br,
    Ret,
    Id,
    UnOpBool(String),
}

#[derive(Clone, Deserialize, Debug, Serialize)]
pub struct Program {
    pub functions: Vec<Function>,
}

#[derive(Clone, Deserialize, Debug, Serialize)]
pub struct Function {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<Param>>,
    pub instrs: Vec<Instruction>,
    pub name: String,
}

#[derive(Clone, Deserialize, Debug, Serialize)]
pub struct Param {
    pub name: String
}

#[derive(Clone, Deserialize, Debug, Serialize)]
pub struct Instruction {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dest: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub op: Option<OpCode>,
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
      "jmp" => OpCode::Jmp,
      "br" => OpCode::Br,
      "ret" => OpCode::Ret,
      "id" => OpCode::Id,
      "print" => OpCode::Print,
      "call" => OpCode::Call,
      _ => panic!("Unknown instruction")
    }
  }
}
