use slotmap::SlotMap;

use std::{
  any::Any,
  collections::HashMap,
};

use crate as kir;
use kir::*;

new_key_type! {
  "rule" => pub struct RuleId;
}

#[derive(Debug, Clone, SExpr, OpIO)]
pub struct Rule {
  #[pp(open = 1)]
  pub name: String,
  #[pp(list_ml)]
  #[pp(close=-1)]
  pub ops: Vec<Op>,
}

impl Rule {
  pub fn with_name(name: String) -> Self {
    Rule { name, ops: vec![] }
  }
}

#[derive(Default)]
pub struct RuleResolver {
  pub map: SlotMap<RuleId, Rule>,
  pub resolver: HashMap<String, RuleId>,
}

impl Resolver for RuleResolver {
  fn name(&self) -> &'static str {
    "rule"
  }
  fn resolve(&mut self, strs: &[String]) -> Result<Box<dyn Any>, String> {
    if strs.len() != 1 {
      return Err("Expected 1 arg for RuleResolver".to_string());
    }
    let name = strs[0].clone();
    if let Some(rule_id) = self.resolver.get(&name) {
      return Ok(Box::new(*rule_id) as Box<dyn Any>);
    } else {
      // insert the rule name into the resolver
      let rule = Rule::with_name(name.clone());
      let rule_id = self.map.insert(rule);
      self.resolver.insert(name, rule_id);
      Ok(Box::new(rule_id) as Box<dyn Any>)
    }
  }
  fn update_by_name(
    &mut self,
    name: String,
    value: Box<dyn Any>,
  ) -> Result<(), String> {
    let rule = *value.downcast::<Rule>().unwrap();
    if let Some(&rule_id) = self.resolver.get(&name) {
      self.map[rule_id] = rule;
    } else {
      let rule_id = self.map.insert(rule);
      self.resolver.insert(name, rule_id);
    }
    Ok(())
  }
}

impl Parse for RuleId {
  fn parse(parser: &mut Parser) -> Result<Self, String> {
    let rule_name = parser.expect(Token::ValueId)?;
    parser.parse_by_resolver::<RuleId, RuleResolver>(
      "rule",
      &[rule_name.to_string()],
    )
  }
}

impl HasResolver for Rule {
  type Resolver = RuleResolver;

  fn name(&self) -> String {
    "rule".to_string()
  }
}

pub struct RuleDumper {
  pub map: SlotMap<RuleId, Rule>,
}

impl Dumper for RuleDumper {
  fn name(&self) -> &'static str {
    "rule"
  }
  fn dump(&mut self, rule_id: Box<dyn Any>) -> String {
    let rule_id = *rule_id.downcast::<RuleId>().unwrap();
    let rule_name = self.map[rule_id].name.clone();
    format!("{}", rule_name)
  }
}

impl RuleDumper {
  pub fn from(map: SlotMap<RuleId, Rule>) -> Self {
    RuleDumper { map }
  }
}


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
  #[pp(map, skip, resolver=ValueResolver, dumper=ValueDumper) ]
  pub values: ValueMap,
  #[pp(open=1)]
  #[pp(surrounded = "inputs")]
  pub inputs: Vec<ValueId>,
  #[pp(nl)]
  #[pp(surrounded = "outputs")]
  pub outputs: Vec<ValueId>,
  #[pp(nl)]
  #[pp(surrounded = "rules")]
  #[pp(close=-1)]
  #[pp(close=-1)]
  #[pp(map, resolver=RuleResolver, dumper=RuleDumper)]
  pub _rules: SlotMap<RuleId, Rule>,
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
      _rules: SlotMap::with_key(),
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

    let mut r0 = Rule::with_name("r0".to_string());
    r0.ops.push(Op::Add(AddOp { res: c, a, b }));
    module._rules.insert(r0);

    let mut r1 = Rule::with_name("r1".to_string());
    r1.ops.push(Op::Add(AddOp { res: c, a, b }));
    module._rules.insert(r1);


    let mut printer = Printer::new();
    printer.print(&module);
    println!("{}", printer.buf);
  }
}
