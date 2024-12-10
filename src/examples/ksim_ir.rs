use crate as kir;
use crate::*;

use std::ops::Range;

use num::BigInt;

#[rustfmt::skip]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, ParsePrint)]
pub enum Cmp {
    Eq, Ne,
    Slt, Sle, Sgt, Sge,
    Ult, Ule, Ugt, Uge,
}
// impl Cmp {
//     pub fn to_str(self) -> &'static str {
//         match self {
//             Cmp::Eq => "eq",
//             Cmp::Ne => "ne",
//             Cmp::Slt => "slt",
//             Cmp::Sle => "sle",
//             Cmp::Sgt => "sgt",
//             Cmp::Sge => "sge",
//             Cmp::Ult => "ult",
//             Cmp::Ule => "ule",
//             Cmp::Ugt => "ugt",
//             Cmp::Uge => "uge",
//         }
//     }
// }

#[rustfmt::skip]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, ParsePrint)]
pub enum Prim {
    Add, Sub, Mul,
    Divu, Divs, Modu, Mods,
    And, Or, Xor, 
    Shl, Lshr, Ashr,
    Rep
}

// impl Prim {
//     pub fn to_str(self) -> &'static str {
//         match self {
//             Prim::Add => "add",
//             Prim::Sub => "sub",
//             Prim::Mul => "mul",
//             Prim::Divu => "divu",
//             Prim::Divs => "divs",
//             Prim::Modu => "modu",
//             Prim::Mods => "mods",
//             Prim::And => "and",
//             Prim::Or => "or",
//             Prim::Xor => "xor",
//             // Prim::Not => "not",
//             Prim::Shl => "shl",
//             Prim::Lshr => "lshr",
//             Prim::Ashr => "ashr",
//             Prim::Rep => "rep",
//         }
//     }
// }

#[derive(Debug, OpIO, Clone, ParsePrint)]
pub struct CmpOp {
  #[opio(output)]
  pub res: ValueId,
  #[pp(punct = "=")]
  #[opio(attr)]
  pub cmp: Cmp,
  #[opio(input)]
  pub inputs: [ValueId; 2],
}

#[derive(Debug, OpIO, Clone, ParsePrint)]
pub struct PrimOp {
  #[opio(output)]
  pub res: ValueId,
  #[pp(punct = "=")]
  #[opio(attr)]
  pub prim: Prim,
  #[opio(input)]
  pub inputs: Vec<ValueId>,
}

#[derive(Debug, OpIO, Clone, ParsePrint)]
pub struct CValueOp {
  #[opio(output)]
  pub res: ValueId,
  #[pp(punct = "=")]
  #[opio(attr)]
  pub value: BigInt,
}

#[derive(Debug, OpIO, Clone, ParsePrint)]
pub struct CArrayOp {
  #[opio(output)]
  pub res: ValueId,
  #[pp(punct = "=")]
  #[opio(attr)]
  pub inputs: Vec<BigInt>,
}

#[derive(Debug, OpIO, Clone, ParsePrint)]
pub struct ArrayOp {
  #[opio(output)]
  pub res: ValueId,
  #[pp(punct = "=")]
  #[opio(input)]
  pub inputs: Vec<ValueId>,
}

// ArrayGetOp
//
// res = array[index]
// where array is an array of values
// index is an integer
#[derive(Debug, OpIO, Clone, ParsePrint)]
pub struct ArrGetOp {
  #[opio(output)]
  pub res: ValueId,
  #[pp(punct = "=")]
  #[opio(input)]
  pub array: ValueId,
  #[pp(kw = "in")]
  #[opio(input)]
  pub index: ValueId,
}

/// SelectOp
///
/// res = conds[cond]
/// where len(conds) = 2 ** width(cond)
#[derive(Debug, OpIO, Clone, ParsePrint)]
pub struct SelectOp {
  #[opio(output)]
  pub res: ValueId,
  #[pp(punct = "=")]
  #[opio(input)]
  pub sel: ValueId,
  #[pp(kw = "in")]
  #[opio(input)]
  pub cands: Vec<ValueId>,
}

/// SliceOp
///
/// res = array[range]
/// array can be either an array or a value
/// if array is a value, it is treated as an array of width 1
#[derive(Debug, OpIO, Clone, ParsePrint)]
pub struct SliceOp {
  #[opio(output)]
  pub res: ValueId,
  #[pp(punct = "=")]
  #[opio(input)]
  pub input: ValueId,
  #[pp(punct = "[", punct_ = "]")]
  #[opio(attr)]
  pub range: Range<u32>,
}

#[derive(Debug, OpIO, Clone, ParsePrint)]
pub struct MemOp {
  #[opio(output)]
  pub read_ref: ValueId,
  #[opio(output)]
  pub write_ref: ValueId,
  pub syn_name: Option<String>,
}

#[derive(Debug, OpIO, Clone, ParsePrint)]
pub struct MemWriteOp {
  #[opio(input)]
  pub write_ref: ValueId,
  #[opio(input)]
  pub addr: ValueId,
  #[opio(input)]
  pub data: ValueId,
  #[opio(input)]
  pub enable: Option<ValueId>,
}

#[derive(Debug, OpIO, Clone, ParsePrint)]
pub struct RegOp {
  #[opio(output)]
  pub res: ValueId,
  #[pp(punct = "=")]
  #[opio(input)]
  pub input: ValueId,
  #[pp(kw = "clock")]
  #[opio(input)]
  pub clock: ValueId,
  #[pp(kw = "name")]
  pub name: Option<String>,
  #[pp(kw = "reset")]
  #[opio(input)]
  pub reset: Option<ValueId>,
  #[pp(kw = "rstval")]
  #[opio(input)]
  pub reset_value: Option<ValueId>,
}

#[derive(Debug, OpIO, Clone, ParsePrint)]
pub struct InstOp {
  #[opio(output)]
  pub res: Vec<ValueId>,
  #[pp(punct = "=")]
  pub inst: String,
  #[pp(kw = "of")]
  pub module: String,
  #[opio(input)]
  pub inputs: Vec<ValueId>,
}

#[derive(Debug, OpIO, Clone, ParsePrint)]
pub struct ConcatOp {
  #[opio(output)]
  pub res: ValueId,
  #[pp(punct = "=")]
  #[opio(input)]
  #[pp(left = "{", right = "}", sep = ",")]
  pub inputs: Vec<ValueId>,
}

#[derive(Debug, OpIO, Clone, ParsePrint)]
pub struct CallOp {
  #[opio(output)]
  pub res: Vec<ValueId>,
  #[pp(punct = "=")]
  #[opio(attr)]
  pub func: String,
  #[opio(input)]
  pub inputs: Vec<ValueId>,
}

#[derive(Debug, OpIO, Clone, ParsePrint)]
pub struct NopOp {
  #[opio(output)]
  pub res: ValueId,
  #[pp(punct = "=")]
  #[opio(input)]
  pub input: ValueId,
}

#[derive(Debug, OpIO, Clone, ParsePrint)]
pub struct StateOp {
  #[opio(output)]
  pub read_ref: ValueId,
  #[opio(output)]
  pub write_ref: ValueId,
  pub name: Option<String>,
}

#[derive(Debug, OpIO, Clone, ParsePrint)]
pub struct ReadOp {
  #[opio(output)]
  pub res: ValueId,
  #[pp(punct = "<=")]
  #[opio(input)]
  pub read_ref: ValueId,
}

#[derive(Debug, OpIO, Clone, ParsePrint)]
pub struct WriteOp {
  #[opio(input)]
  pub write_ref: ValueId,
  #[pp(punct = "<=")]
  #[opio(input)]
  pub data: ValueId,
  #[opio(input)]
  pub enable: Option<ValueId>,
}

// Operation types
//
// Use Box to reduce the size of the enum
#[derive(Debug, OpIO, Clone, ParsePrint)]
pub enum Op {
  Nop(Box<NopOp>),
  Cmp(Box<CmpOp>),
  Prim(Box<PrimOp>),
  CValue(Box<CValueOp>),
  CArray(Box<CArrayOp>),
  Array(Box<ArrayOp>),
  ArrGet(Box<ArrGetOp>),
  Select(Box<SelectOp>),
  Slice(Box<SliceOp>),
  Concat(Box<ConcatOp>),
  Call(Box<CallOp>),
  Mem(Box<MemOp>),
  MemWrite(Box<MemWriteOp>),
  Reg(Box<RegOp>),
  State(Box<StateOp>),
  Read(Box<ReadOp>),
  Write(Box<WriteOp>),
  Inst(Box<InstOp>),
}

#[derive(Debug, ParsePrint)]
pub struct Module {
  #[pp(kw = "module")]
  pub name: String,
  #[pp(value_map)]
  pub values: ValueMap,
  pub inputs: Vec<ValueId>,
  #[pp(punct = "->")]
  pub outputs: Vec<ValueId>,
  #[pp(semi_list)]
  pub ops: Vec<Op>,
}

#[derive(Debug, ParsePrint)]
pub struct Design {
  #[pp(kw = "design")]
  pub top: String,
  #[pp(semi_list)]
  pub modules: Vec<Module>,
}

#[cfg(test)]
mod tests {
  use core::panic;
  use kir::{Parser, Printer};

  use super::*;
  #[test]
  fn size_of_op() {
    eprintln!("CmpOp: {}", std::mem::size_of::<CmpOp>());
    eprintln!("PrimOp: {}", std::mem::size_of::<PrimOp>());
    eprintln!("CValueOp: {}", std::mem::size_of::<CValueOp>());
    eprintln!("CArrayOp: {}", std::mem::size_of::<CArrayOp>());
    eprintln!("ArrayOp: {}", std::mem::size_of::<ArrayOp>());
    eprintln!("ArrGetOp: {}", std::mem::size_of::<ArrGetOp>());
    eprintln!("SelectOp: {}", std::mem::size_of::<SelectOp>());
    eprintln!("SliceOp: {}", std::mem::size_of::<SliceOp>());
    eprintln!("MemOp: {}", std::mem::size_of::<MemOp>());
    eprintln!("MemWriteOp: {}", std::mem::size_of::<MemWriteOp>());
    eprintln!("RegOp: {}", std::mem::size_of::<RegOp>());
    eprintln!("InstOp: {}", std::mem::size_of::<InstOp>());
    eprintln!("Op: {}", std::mem::size_of::<Op>());
    assert!(std::mem::size_of::<Op>() <= 24);
  }

  #[test]
  fn parse_print() {
    let mut parser = Parser::new("module \"test\" (%a:i32 %b:i32) -> (%res:i32) { prim %res:i32 = add (%a:i32 %b:i32) }");
    let module = match parser.parse::<Module>() {
      Ok(module) => module,
      Err(err) => {
        parser.report_error("input", &err);
        panic!("Error parsing module");
      }
    };
    let mut printer = Printer::new();
    printer.print(&module);
    let res = printer.buf;
    eprintln!("{}", res);
    let mut parser = Parser::new(&res);
    let module2 = match parser.parse::<Module>() {
      Ok(module) => module,
      Err(err) => {
        parser.report_error("input", &err);
        panic!("Error parsing module");
      }
    };
    assert_eq!(module.name, module2.name);
    assert_eq!(module.ops.len(), module2.ops.len());
    assert_eq!(module.values.len(), module2.values.len());
    eprintln!("{:#?}", module2);
  }
}
