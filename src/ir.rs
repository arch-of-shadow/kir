use std::{
  fmt::Debug,
  hash::Hash,
  ops::{Deref, DerefMut},
  str::FromStr,
};

use crate::{new_key_type, IdFor};
use slotmap::SlotMap;

new_key_type! {
    pub struct ValueId; => Value
}

// TODO: its strange to have Type inside kir, but it is coupled with Value
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
  // software integer type, (rust i32)
  Integer,
  // unsigned int
  UInt(u32),
  // signed int
  SInt(u32),
  // probe, rwprobe, NOT HARDWARE
  Ref(u32),
  // vector type
  Vector(Box<Type>, u32),
  // bundle type
  Bundle(String, Vec<(String, Type, bool)>),
  // enum type
  Enum(String, Vec<(String, Type)>),
}

impl Type {
  pub fn unit() -> Type {
    Type::UInt(0)
  }
  pub fn uint(width: u32) -> Type {
    Type::UInt(width)
  }
  pub fn sint(width: u32) -> Type {
    Type::SInt(width)
  }
  pub fn new_ref(width: u32) -> Type {
    Type::Ref(width)
  }
  pub fn vector(base: Type, depth: u32) -> Type {
    Type::Vector(Box::new(base), depth)
  }
  pub fn bundle(name: String, fields: Vec<(String, Type, bool)>) -> Type {
    // must sort fields by name
    let mut fields = fields;
    fields.sort_by(|a, b| a.0.cmp(&b.0));
    Type::Bundle(name, fields)
  }
  pub fn union(name: String, variants: Vec<(String, Option<Type>)>) -> Type {
    Type::Enum(
      name,
      variants
        .into_iter()
        .map(|(name, ty)| (name, ty.unwrap_or(Type::unit())))
        .collect(),
    )
  }
  pub fn int_width(&self) -> u32 {
    match self {
      Type::UInt(width) => *width,
      Type::SInt(width) => *width,
      _ => panic!("Type {self:?} is not an integer"),
    }
  }
  pub fn int_to_ref(&self) -> Self {
    match self {
      Type::UInt(width) => Type::Ref(*width),
      _ => panic!("Type {self:?} is not an integer"),
    }
  }
  pub fn ref_to_int(&self) -> Self {
    match self {
      Type::Ref(width) => Type::UInt(*width),
      _ => panic!("Type {self:?} is not a reference"),
    }
  }
  pub fn ref_width(&self) -> u32 {
    match self {
      Type::Ref(width) => *width,
      _ => panic!("Type is not a reference"),
    }
  }
  pub fn vector_elem_width(&self) -> u32 {
    match self {
      Type::Vector(base, _) => base.int_width(),
      _ => panic!("Type is not an array"),
    }
  }
  pub fn vector_depth(&self) -> u32 {
    match self {
      Type::Vector(_, depth) => *depth,
      _ => panic!("Type is not an array"),
    }
  }
}

impl ToString for Type {
  fn to_string(&self) -> String {
    match self {
      Type::Integer => "integer".to_string(),
      Type::UInt(width) => format!("i{}", width),
      Type::SInt(width) => format!("s{}", width),
      Type::Ref(width) => format!("r{}", width),
      Type::Vector(base, depth) => format!("{}x{}", base.to_string(), depth),
      Type::Bundle(name, _fields) => format!(
        "{name}"
        // "{{{}}}",
        // fields
        //   .iter()
        //   .map(|(name, ty, flip)| {
        //     format!(
        //       "{}{}: {}",
        //       if *flip { "flip " } else { "" },
        //       name,
        //       ty.to_string()
        //     )
        //   })
        //   .collect::<Vec<String>>()
        //   .join(", ")
      ),
      Type::Enum(name, _variants) => format!(
        "{name}" /* "{{|{}|}}",
                  * variants
                  *   .iter()
                  *   .map(|(name, ty)| format!("{name}: {}",
                  * ty.to_string()))
                  *   .collect::<Vec<String>>()
                  *   .join(", ") */
      ),
    }
  }
}
impl FromStr for Type {
  type Err = String;
  fn from_str(s: &str) -> Result<Self, Self::Err> {
    // int type: i32, i64, ...
    // arr type: i32x2, i64x8 ...

    // if starts with {, and ends with }, then it is a struct
    if s.starts_with('{') && s.ends_with('}') {
      let fields = s[1..s.len() - 1]
        .split(',')
        .map(|f| f.split_once(':').unwrap())
        .map(|(name, ty)| {
          let (name, flip) = if let Some((_, name)) = name.split_once(" ") {
            if name == "flip" {
              (name.to_string(), true)
            } else {
              return Err(format!("Invalid field name: {}", name));
            }
          } else {
            (name.to_string(), false)
          };
          Ok((name, Type::from_str(ty)?, flip))
        })
        .collect::<Result<Vec<(String, Type, bool)>, String>>()?;
      Ok(Type::bundle("bundle".to_string(), fields))
    } else if s.starts_with("{|") && s.ends_with("|}") {
      let variants = s[2..s.len() - 2]
        .split(',')
        .map(|v| {
          if let Some((name, ty)) = v.split_once(':') {
            if let Ok(ty) = Type::from_str(ty) {
              Ok((name.to_string(), Some(ty)))
            } else {
              return Err(format!("Invalid type: {}", ty));
            }
          } else {
            Ok((v.to_string(), None))
          }
        })
        .collect::<Result<Vec<(String, Option<Type>)>, String>>()?;
      Ok(Type::union("union".to_string(), variants))
    } else {
      let mut parts = s.split('x');
      let base_part = parts
        .next()
        .ok_or_else(|| "Empty type string".to_string())?;
      if let Some(depth) = parts.next() {
        let base = Type::from_str(base_part)?;
        let depth =
          depth.parse().map_err(|e| format!("Invalid depth: {}", e))?;
        Ok(Type::Vector(Box::new(base), depth))
      } else {
        if base_part == "integer" {
          Ok(Type::Integer)
        } else if base_part.starts_with('i') {
          Ok(Type::UInt(base_part[1..].parse().unwrap()))
        } else if base_part.starts_with('r') {
          Ok(Type::Ref(base_part[1..].parse().unwrap()))
        } else if base_part.starts_with('s') {
          Ok(Type::SInt(base_part[1..].parse().unwrap()))
        } else {
          Err(format!("Invalid type string: {}", s))
        }
      }
    }
  }
}

#[derive(Debug, Clone)]
pub struct Value {
  pub ty: Option<Type>,
  pub name: Option<String>,
}

impl Value {
  pub fn new(ty: Type, name: Option<String>) -> Self {
    Value { ty: Some(ty), name }
  }

  pub fn new_wo_ty(name: Option<String>) -> Self {
    Value { ty: None, name }
  }
}

impl ValueId {
  pub fn ty(&self, t: &SlotMap<ValueId, Value>) -> Option<Type> {
    t[*self].ty.clone()
  }
  pub fn name<'r>(&self, t: &'r SlotMap<ValueId, Value>) -> &'r Option<String> {
    &t[*self].name
  }
  pub fn name_mut<'r>(
    &self,
    t: &'r mut SlotMap<ValueId, Value>,
  ) -> &'r mut Option<String> {
    &mut t[*self].name
  }
}

pub type ValueMap = SlotMap<ValueId, Value>;

pub trait OpIO {
  fn num_inputs(&self) -> usize;
  fn input(&self, i: usize) -> ValueId;
  fn input_mut(&mut self, i: usize) -> &mut ValueId;
  fn inputs(&self) -> impl Iterator<Item = ValueId> + '_ {
    (0..self.num_inputs()).map(move |i| self.input(i))
  }
  fn map_inputs(&mut self, mut f: impl FnMut(ValueId) -> ValueId) {
    for i in 0..self.num_inputs() {
      *self.input_mut(i) = f(self.input(i));
    }
  }
  fn num_outputs(&self) -> usize;
  fn output(&self, i: usize) -> ValueId;
  fn output_mut(&mut self, i: usize) -> &mut ValueId;
  fn outputs(&self) -> impl Iterator<Item = ValueId> + '_ {
    (0..self.num_outputs()).map(move |i| self.output(i))
  }
  fn map_outputs(&mut self, mut f: impl FnMut(ValueId) -> ValueId) {
    for i in 0..self.num_outputs() {
      *self.output_mut(i) = f(self.output(i));
    }
  }
  fn values(&self) -> impl Iterator<Item = ValueId> + '_ {
    self.inputs().chain(self.outputs())
  }
  fn map_values(&mut self, mut f: impl FnMut(ValueId) -> ValueId) {
    self.map_inputs(&mut f);
    self.map_outputs(&mut f);
  }
  fn attr_eq(&self, _rhs: &Self) -> bool {
    true
  }
  fn attr_hash<H: std::hash::Hasher>(&self, _state: &mut H) {}
}

impl OpIO for ValueId {
  fn num_inputs(&self) -> usize {
    0
  }
  fn input(&self, _i: usize) -> ValueId {
    panic!("ValueId has no inputs");
  }
  fn input_mut(&mut self, _i: usize) -> &mut ValueId {
    panic!("ValueId has no inputs");
  }
  fn num_outputs(&self) -> usize {
    1
  }
  fn output(&self, i: usize) -> ValueId {
    assert_eq!(i, 0);
    *self
  }
  fn output_mut(&mut self, i: usize) -> &mut ValueId {
    assert_eq!(i, 0);
    self
  }
  fn attr_eq(&self, rhs: &Self) -> bool {
    *self == *rhs
  }
  fn attr_hash<H: std::hash::Hasher>(&self, state: &mut H) {
    std::hash::Hash::hash(&self, state);
  }
}

impl<T: OpIO> OpIO for Box<T> {
  fn num_inputs(&self) -> usize {
    self.deref().num_inputs()
  }
  fn input(&self, i: usize) -> ValueId {
    self.deref().input(i)
  }
  fn input_mut(&mut self, i: usize) -> &mut ValueId {
    self.deref_mut().input_mut(i)
  }
  fn num_outputs(&self) -> usize {
    self.deref().num_outputs()
  }
  fn output(&self, i: usize) -> ValueId {
    self.deref().output(i)
  }
  fn output_mut(&mut self, i: usize) -> &mut ValueId {
    self.deref_mut().output_mut(i)
  }
  fn attr_eq(&self, rhs: &Self) -> bool {
    self.deref().attr_eq(rhs.deref())
  }
  fn attr_hash<H: std::hash::Hasher>(&self, state: &mut H) {
    self.deref().attr_hash(state)
  }
}

impl<T: OpIO> OpIO for Vec<T> {
  fn num_inputs(&self) -> usize {
    self.iter().map(|t| t.num_inputs()).sum()
  }
  fn input(&self, i: usize) -> ValueId {
    let mut offset = 0;
    for t in self {
      if i < offset + t.num_inputs() {
        return t.input(i - offset);
      }
      offset += t.num_inputs();
    }
    panic!("Index out of bounds");
  }
  fn input_mut(&mut self, i: usize) -> &mut ValueId {
    let mut offset = 0;
    for t in self {
      if i < offset + t.num_inputs() {
        return t.input_mut(i - offset);
      }
      offset += t.num_inputs();
    }
    panic!("Index out of bounds");
  }
  fn num_outputs(&self) -> usize {
    self.iter().map(|t| t.num_outputs()).sum()
  }
  fn output(&self, i: usize) -> ValueId {
    let mut offset = 0;
    for t in self {
      if i < offset + t.num_outputs() {
        return t.output(i - offset);
      }
      offset += t.num_outputs();
    }
    panic!("Index out of bounds");
  }
  fn output_mut(&mut self, i: usize) -> &mut ValueId {
    let mut offset = 0;
    for t in self {
      if i < offset + t.num_outputs() {
        return t.output_mut(i - offset);
      }
      offset += t.num_outputs();
    }
    panic!("Index out of bounds");
  }
  fn attr_eq(&self, rhs: &Self) -> bool {
    self.iter().zip(rhs.iter()).all(|(a, b)| a.attr_eq(b))
  }
  fn attr_hash<H: std::hash::Hasher>(&self, state: &mut H) {
    for t in self {
      t.attr_hash(state);
    }
  }
}

impl<T: OpIO> OpIO for Option<T> {
  fn num_inputs(&self) -> usize {
    self.as_ref().map(|t| t.num_inputs()).unwrap_or(0)
  }

  fn input(&self, i: usize) -> ValueId {
    self.as_ref().unwrap().input(i)
  }

  fn input_mut(&mut self, i: usize) -> &mut ValueId {
    self.as_mut().unwrap().input_mut(i)
  }

  fn num_outputs(&self) -> usize {
    self.as_ref().map(|t| t.num_outputs()).unwrap_or(0)
  }

  fn output(&self, i: usize) -> ValueId {
    self.as_ref().unwrap().output(i)
  }

  fn output_mut(&mut self, i: usize) -> &mut ValueId {
    self.as_mut().unwrap().output_mut(i)
  }

  fn attr_eq(&self, rhs: &Self) -> bool {
    match (self, rhs) {
      (None, None) => true,
      (Some(a), Some(b)) => a.attr_eq(b),
      _ => false,
    }
  }

  fn attr_hash<H: std::hash::Hasher>(&self, state: &mut H) {
    self.as_ref().map(|t| t.attr_hash(state));
  }
}

#[derive(Debug, Clone, Copy)]
pub struct AttrView<'op, T: OpIO>(pub &'op T);
impl<'op, T: OpIO> AttrView<'op, T> {
  pub fn new(op: &'op T) -> Self {
    Self(op)
  }
}
impl<'op, T: OpIO> std::ops::Deref for AttrView<'op, T> {
  type Target = T;
  fn deref(&self) -> &Self::Target {
    self.0
  }
}
impl<'op, T: OpIO> std::cmp::PartialEq for AttrView<'op, T> {
  fn eq(&self, rhs: &Self) -> bool {
    self.0.attr_eq(rhs.0)
  }
}
impl<'op, T: OpIO> std::cmp::Eq for AttrView<'op, T> {}
impl<'op, T: OpIO> std::hash::Hash for AttrView<'op, T> {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    self.0.attr_hash(state)
  }
}
