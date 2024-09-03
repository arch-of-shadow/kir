use kir::*;

#[derive(Debug, Clone, SExpr, OpIO)]
pub struct AddOp {
  #[opio(output)]
  pub res: ValueId,
  #[pp(open)]
  pub a: ValueId,
  #[pp(close)]
  pub b: ValueId,
}

#[derive(Debug, Clone, OpIO, SExpr)]
pub enum Op {
  Add(AddOp),
}

#[derive(Debug, Clone, SExpr, OpIO)]
pub struct Module {
  #[pp(open=1)]
  #[pp(kw = "module")]
  pub name: String,
  #[pp(value_map)]
  pub values: ValueMap,
  #[pp(open=1)]
  #[pp(surrounded = "inputs")]
  pub inputs: Vec<ValueId>,
  #[pp(nl)]
  #[pp(surrounded = "outputs")]
  pub outputs: Vec<ValueId>,
  #[pp(nl)]
  #[pp(surrounded = "ops")]
  #[pp(list_ml)]
  #[pp(close=-1)]
  #[pp(close=-1)]
  pub ops: Vec<Op>,
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_add_op() {
    let mut module = Module {
      name: "add".to_string(),
      values: ValueMap::with_key(),
      inputs: vec![],
      outputs: vec![],
      ops: vec![],
    };

    let a = module.values.insert(Value {
      ty: Type::Int(4),
      name: Some("a".to_string()),
    });
    let b = module.values.insert(Value {
      ty: Type::Int(4),
      name: Some("b".to_string()),
    });
    let c = module.values.insert(Value {
      ty: Type::Int(4),
      name: Some("c".to_string()),
    });

    module.inputs.push(a);
    module.inputs.push(b);
    module.outputs.push(c);

    let add = AddOp { res: c, a, b };
    module.ops.push(Op::Add(add));

    let mut printer = Printer::new();
    printer.print(&module);
    println!("{}", printer.buf);
  }
}
