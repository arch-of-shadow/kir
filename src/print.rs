use std::{collections::HashMap, fmt::Display, ops::Range};

use num::{BigInt, BigUint};
use slotmap::SecondaryMap;

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
    pub fn next_unnamed(&mut self) -> ValueName<'r> {
        let id = self.next_value_id;
        self.next_value_id += 1;
        ValueName::Unnamed(id)
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
            None => self.next_unnamed(),
        };
        self.resolved.insert(vid, res);
        res
    }
}

pub struct Printer<'v> {
    pub buf: String,
    pub ident: i32,
    pub space: bool,
    pub vp: Option<ValuePrinter<'v>>,
}

impl<'v> Printer<'v> {
    pub fn new() -> Self {
        Printer {
            buf: String::new(),
            ident: 0,
            space: false,
            vp: None,
        }
    }
    pub fn write_fmt(&mut self, fmt: std::fmt::Arguments) {
        use std::fmt::Write;
        if self.space {
            self.buf.push_str(" ");
        }
        self.buf.write_fmt(fmt).unwrap();
        self.space = true;
    }
    pub fn newline(&mut self) {
        self.space = false;
        self.buf.push_str("\n");
        for _ in 0..self.ident {
            self.buf.push_str("  ");
        }
    }
    pub fn ident(&mut self, diff: i32) { self.ident += diff; }
    pub fn space(&mut self) { self.space = true; }
    pub fn print_unescaped_str(&mut self, s: &str) {
        self.write_fmt(format_args!("\"{}\"", escape_str(s)));
    }
    pub fn print<T: Print>(&mut self, item: &'v T) { item.print(self); }
    pub fn print_value(&mut self, vid: ValueId) {
        let vp = self.vp.as_mut().unwrap();
        let name = vp.resolve_name(vid);
        let ty = vp.values[vid].ty.to_string();
        match name {
            ValueName::Named(name, 0) => self.write_fmt(format_args!("%{}:{}", name, ty)),
            ValueName::Named(name, id) => self.write_fmt(format_args!("%{}_{}:{}", name, id, ty)),
            ValueName::Unnamed(id) => self.write_fmt(format_args!("%{}:{}", id, ty)),
        }
    }
    pub fn print_list<'r: 'v, T: Print + 'r>(
        &mut self,
        left: &str,
        sep: &str,
        right: &str,
        newline: bool,
        last: bool,
        list: impl 'r + IntoIterator<Item = &'r T>,
    ) {
        let mut first = true;
        self.write_fmt(format_args!("{}", left));
        if newline {
            self.ident(1);
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
            self.ident(-1);
        }
        if !first && newline {
            self.newline();
        }
        self.space = false;
        self.write_fmt(format_args!("{}", right));
    }
    pub fn set_printer(&mut self, printer: Option<ValuePrinter<'v>>) -> Option<ValuePrinter<'v>> {
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
    fn ir_dump(&self) -> String { ir_dump(self) }
    fn ir_dump_with(&self, values: &ValueMap) -> String { ir_dump_with(values, self) }
}

impl Print for String {
    fn print<'p>(&'p self, p: &mut Printer<'p>) { p.print_unescaped_str(self); }
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
impl_print_for_number!(bool, i8, i16, i32, i64, u8, u16, u32, u64, f32, f64, isize, usize);
impl_print_for_number!(BigUint, BigInt);

impl Print for Type {
  fn print<'p>(&'p self, p: &mut Printer<'p>) {
    write!(p, "{}", self.to_string());
  }
}

impl Print for ValueId {
    fn print<'p>(&'p self, p: &mut Printer<'p>) { p.print_value(*self); }
}

impl<T: Print> Print for Vec<T> {
    fn print<'p>(&'p self, p: &mut Printer<'p>) {
        p.print_list("(", "", ")", false, false, self.iter());
    }
}
impl<T: Print, const N: usize> Print for [T; N] {
    fn print<'p>(&'p self, p: &mut Printer<'p>) {
        p.print_list("(", "", ")", true, false, self.iter());
    }
}
impl<T: Print> Print for Option<T> {
    fn print<'p>(&'p self, p: &mut Printer<'p>) {
        p.print_list("(", "", ")", true, false, self.iter())
    }
}
impl<T: Print> Print for Box<T> {
    fn print<'p>(&'p self, p: &mut Printer<'p>) { self.as_ref().print(p); }
}
impl<T: Print> Print for Range<T> {
    fn print<'p>(&'p self, p: &mut Printer<'p>) {
        self.start.print(p);
        write!(p, "..");
        self.end.print(p);
    }
}
