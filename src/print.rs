use std::{collections::HashMap, fmt::Display, ops::Range};

use indexmap::IndexMap;
use num::{BigInt, BigUint};
use slotmap::SecondaryMap;
use std::hash::Hash;

use crate::ir::*;

fn escape_str(s: &str) -> String {
  let mut res = String::new();
  for c in s.chars() {
    match c {
      '\\' => res.push_str("\\\\"),
      '"' => res.push_str("\\\""),
      '\n' => res.push_str("\\n"),
      '\r' => res.push_str("\\r"),
      '\t' => res.push_str("\\t"),
      _ => res.push(c),
    }
  }
  res
}

pub struct ValuePrinter<'r> {
  pub values: &'r ValueMap,
  pub used: HashMap<String, usize>,
  pub resolved: SecondaryMap<ValueId, ValueName<'r>>,
  pub next_value_id: usize,
}

#[derive(Clone, Copy)]
pub enum ValueName<'r> {
  Named(&'r str, usize),
  Unnamed(usize),
}
impl Display for ValueName<'_> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      ValueName::Named(name, 0) => write!(f, "{}", name),
      ValueName::Named(name, n) => write!(f, "{}_{}", name, n),
      ValueName::Unnamed(n) => write!(f, "{}", n),
    }
  }
}

impl<'r> ValuePrinter<'r> {
  pub fn new(values: &'r ValueMap) -> Self {
    ValuePrinter {
      values,
      used: HashMap::new(),
      resolved: SecondaryMap::new(),
      next_value_id: 0,
    }
  }
  pub fn find_next_name(&mut self, name: &str) -> usize {
    if let Some(v) = self.used.get(name) {
      let mut v = *v;
      while self.used.contains_key(&format!("{}_{}", name, v)) {
        v += 1;
      }
      *self.used.get_mut(name).unwrap() = v;
      v
    } else {
      self.used.insert(name.to_string(), 1);
      0
    }
  }
  pub fn next_unnamed(&mut self, vid: ValueId) -> ValueName<'r> {
    // let id = self.next_value_id;
    // self.next_value_id += 1;
    // ValueName::Unnamed(id)

    // instead of using next_value_id, just use the vid as the unnamed id
    // FIXME: this is not a good idea, because the vid might(?) be not unique
    // TODO: add a checker to ensure the vid is unique
    ValueName::Unnamed(vid.as_ffi() as usize)
  }
  pub fn resolve_name(&mut self, vid: ValueId) -> ValueName<'r> {
    if let Some(name) = self.resolved.get(vid) {
      return *name;
    }
    let res = match self.values[vid].name {
      Some(ref name) => {
        let id = self.find_next_name(name);
        ValueName::Named(name, id)
      }
      None => self.next_unnamed(vid),
    };
    self.resolved.insert(vid, res);
    res
  }
}

pub struct Printer<'v> {
  pub buf: String,
  pub indent: i32,
  pub space: bool,
  pub vp: Option<ValuePrinter<'v>>,
}

impl<'v> Printer<'v> {
  pub fn new() -> Self {
    Printer {
      buf: String::new(),
      indent: 0,
      space: false,
      vp: None,
    }
  }
  pub fn write_fmt(&mut self, fmt: std::fmt::Arguments) {
    use std::fmt::Write;
    if self.space && !fmt.to_string().starts_with(')') {
      self.buf.push_str(" ");
      self.space = false;
    }
    self.buf.write_fmt(fmt).unwrap();
    if !fmt.to_string().ends_with('(') {
      self.space = true;
      // println!("space, last fmt is {:?}", fmt);
    } else {
      // println!("no space, last fmt is {:?}", fmt);
    }
  }
  pub fn newline(&mut self) {
    self.space = false;
    self.buf.push_str("\n");
    for _ in 0..self.indent {
      self.buf.push_str("  ");
    }
  }
  pub fn indent(&mut self, diff: i32) {
    self.indent += diff;
  }
  pub fn print_unescaped_str(&mut self, s: &str) {
    self.write_fmt(format_args!("\"{}\"", escape_str(s)));
  }
  pub fn print<T: Print>(&mut self, item: &'v T) {
    item.print(self);
  }
  pub fn print_value(&mut self, vid: ValueId) {
    let vp = self.vp.as_mut().unwrap();
    let name = vp.resolve_name(vid);
    let ty = vp.values[vid].ty.clone();
    match name {
      ValueName::Named(name, 0) => {
        self.write_fmt(format_args!("%{}{}", name, type_str(&ty)))
      }
      ValueName::Named(name, id) => {
        self.write_fmt(format_args!("%{}_{}{}", name, id, type_str(&ty)))
      }
      ValueName::Unnamed(id) => {
        self.write_fmt(format_args!("%{}{}", id, type_str(&ty)))
      }
    }
  }
  pub fn print_list<'r: 'v, T: Print + 'r>(
    &mut self,
    kw: &str,
    left: &str,
    sep: &str,
    right: &str,
    newline: bool,
    last: bool,
    list: impl 'r + IntoIterator<Item = &'r T>,
  ) {
    let mut first = true;
    self.write_fmt(format_args!("{}", left));
    if kw != "" {
      self.write_fmt(format_args!("{}", kw));
    }
    if newline {
      self.indent(1);
    } else {
      self.space = false;
    }
    for item in list {
      if first {
        first = false;
      } else {
        self.buf.push_str(sep);
      }
      if newline {
        self.newline();
      }
      item.print(self);
    }
    if !first && last {
      if last {
        self.buf.push_str(sep);
      }
    }
    if newline {
      self.indent(-1);
    }
    if !first && newline {
      self.newline();
    }
    self.space = false;
    self.write_fmt(format_args!("{}", right));
  }

  pub fn print_list_tuple<'r: 'v, K: Print + 'r, D: Print + 'r>(
    &mut self,
    kw: &str,
    left: &str,
    sep: &str,
    right: &str,
    newline: bool,
    last: bool,
    list: impl 'r + IntoIterator<Item = (&'r K, &'r D)>,
  ) {
    let mut first = true;
    self.write_fmt(format_args!("{}", left));
    if kw != "" {
      self.write_fmt(format_args!("{}", kw));
    }
    if newline {
      self.indent(1);
    } else {
      self.space = false;
    }
    for (k, d) in list {
      if first {
        first = false;
      } else {
        self.buf.push_str(sep);
      }
      if newline {
        self.newline();
      }
      k.print(self);
      write!(self, " : ");
      d.print(self);
    }
    if !first && last {
      if last {
        self.buf.push_str(sep);
      }
    }
    if newline {
      self.indent(-1);
    }
    if !first && newline {
      self.newline();
    }
    self.space = false;
    self.write_fmt(format_args!("{}", right));
  }

  pub fn set_printer(
    &mut self,
    printer: Option<ValuePrinter<'v>>,
  ) -> Option<ValuePrinter<'v>> {
    std::mem::replace(&mut self.vp, printer)
  }
}

pub fn ir_dump(n: &impl Print) -> String {
  let mut p = Printer::new();
  p.print(n);
  p.buf
}
pub fn ir_dump_with(values: &ValueMap, n: &impl Print) -> String {
  let mut p = Printer::new();
  p.vp = Some(ValuePrinter::new(values));
  p.print(n);
  p.buf
}

pub trait Print {
  fn print<'p>(&'p self, p: &mut Printer<'p>);
}

pub trait IRDump: Sized {
  fn ir_dump(&self) -> String;
  fn ir_dump_with(&self, values: &ValueMap) -> String;
}
impl<T: Print + Sized> IRDump for T {
  fn ir_dump(&self) -> String {
    ir_dump(self)
  }
  fn ir_dump_with(&self, values: &ValueMap) -> String {
    ir_dump_with(values, self)
  }
}

impl Print for String {
  fn print<'p>(&'p self, p: &mut Printer<'p>) {
    p.print_unescaped_str(self);
  }
}

macro_rules! impl_print_for_number {
    ($($ty:ty),*) => {
        $(
            impl Print for $ty {
                fn print<'p>(&'p self, p: &mut Printer<'p>) {
                    write!(p, "{}", self);
                }
            }
        )*
    };
}
impl_print_for_number!(
  bool, i8, i16, i32, i64, u8, u16, u32, u64, f32, f64, isize, usize
);
impl_print_for_number!(BigUint, BigInt);

impl Print for Type {
  fn print<'p>(&'p self, p: &mut Printer<'p>) {
    write!(p, "{}", self.to_string());
  }
}

impl Print for ValueId {
  fn print<'p>(&'p self, p: &mut Printer<'p>) {
    p.print_value(*self);
  }
}

impl<T: Print> Print for Vec<T> {
  fn print<'p>(&'p self, p: &mut Printer<'p>) {
    p.print_list("", "(", "", ")", false, false, self.iter());
  }
}

impl<T: Print, const N: usize> Print for [T; N] {
  fn print<'p>(&'p self, p: &mut Printer<'p>) {
    p.print_list("", "(", "", ")", true, false, self.iter());
  }
}

impl<T: Print> Print for Option<T> {
  fn print<'p>(&'p self, p: &mut Printer<'p>) {
    // p.print_list("", "(", "", ")", false, false, self.iter())
    match self {
      Some(v) => v.print(p),
      None => write!(p, "_"),
    }
  }
}

impl<T: Print> Print for Box<T> {
  fn print<'p>(&'p self, p: &mut Printer<'p>) {
    self.as_ref().print(p);
  }
}

impl<T: Print> Print for Range<T> {
  fn print<'p>(&'p self, p: &mut Printer<'p>) {
    self.start.print(p);
    write!(p, "..");
    self.end.print(p);
  }
}

impl<K: Print + Eq + Hash + Clone, D: Print + Clone> Print for IndexMap<K, D> {
  fn print<'p>(&'p self, p: &mut Printer<'p>) {
    p.print_list_tuple("", "{", ",", "}", true, false, self);
  }
}

fn type_str(ty: &Option<Type>) -> String {
  match ty {
    Some(ty) => format!(":{}", ty.to_string()),
    None => "".to_string(),
    // None => ":_tbd".to_string(),
  }
}

const MAX_DEPTH: usize = 1;

impl Print for json::object::Object {
  fn print<'p>(&'p self, p: &mut Printer<'p>) {
    p.write_fmt(format_args!(
      "{}",
      json::stringify(
        json::JsonValue::Object(self.clone()).limit_depth(MAX_DEPTH)
      )
    ));
  }
}

trait ObjectManipulate {
  fn limit_depth(self, depth: usize) -> Self;
}

impl ObjectManipulate for json::JsonValue {
  fn limit_depth(self, depth: usize) -> Self {
    match self {
      json::JsonValue::Object(obj) => {
        if depth > 0 {
          let mut res = json::object::Object::new();
          for (k, v) in obj.iter() {
            res.insert(k, v.clone().limit_depth(depth - 1));
          }
          json::JsonValue::Object(res)
        } else {
          json::JsonValue::String("...".to_string())
        }
      }
      _ => self,
    }
  }
}
