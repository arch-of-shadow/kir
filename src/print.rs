use std::{any::Any, collections::HashMap, ops::Range};

use num::{BigInt, BigUint};
use slotmap::{Key, SlotMap};


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

// similar to the Resolver trait, but for printing
pub trait Dumper: Any {
  fn name(&self) -> &'static str;
  fn dump(&mut self, item: Box<dyn Any>) -> String;
}

pub struct Printer {
    pub buf: String,
    pub ident: i32,
    pub space: bool,
    // pub vp: Option<ValuePrinter<'v>>,
    pub dumpers: HashMap<String, Box<dyn Any>>,
}

impl Printer {
    pub fn new() -> Self {
        Printer {
            buf: String::new(),
            ident: 0,
            space: false,
            dumpers: HashMap::new(),
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
    pub fn print<T: Print>(&mut self, item: &T) { item.print(self); }
    pub fn print_by_dumper<T: 'static, D: Dumper>(&mut self, name: &str, item: T) -> Result<(), String> {
      let dumper = self
      .dumpers
      .get_mut(name)
      .ok_or_else(|| format!("No resolver for {}", name))?
      .downcast_mut::<D>()
      .ok_or_else(|| format!("Resolver type mismatch for {}", name))?;
      let res = dumper.dump(Box::new(item));
      self.write_fmt(format_args!("{}", res));
      Ok(())
    }
    // pub fn print_value(&mut self, vid: ValueId) {
    //     let vp = self.vp.as_mut().unwrap();
    //     let name = vp.resolve_name(vid);
    //     let ty = vp.values[vid].ty.to_string();
    //     match name {
    //         ValueName::Named(name, 0) => self.write_fmt(format_args!("%{}:{}", name, ty)),
    //         ValueName::Named(name, id) => self.write_fmt(format_args!("%{}_{}:{}", name, id, ty)),
    //         ValueName::Unnamed(id) => self.write_fmt(format_args!("%{}:{}", id, ty)),
    //     }
    // }
    pub fn print_list<'r, T: Print + 'r>(
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

    // pub fn set_printer(&mut self, printer: Option<ValuePrinter<'v>>) -> Option<ValuePrinter<'v>> {
    //     std::mem::replace(&mut self.vp, printer)
    // }

    pub fn add_dumper(&mut self, name: &str, dumper:  Box<dyn Any>) {
        self.dumpers.insert(name.to_string(), dumper);
    }
}

pub fn ir_dump(n: &impl Print) -> String {
    let mut p = Printer::new();
    p.print(n);
    p.buf
}
// pub fn ir_dump_with(values: &ValueMap, n: &impl Print) -> String {
//     let mut p = Printer::new();
//     p.vp = Some(ValuePrinter::new(values));
//     p.print(n);
//     p.buf
// }

pub trait Print {
    fn print(&self, p: &mut Printer);
}

pub trait IRDump: Sized {
    fn ir_dump(&self) -> String;
    // fn ir_dump_with(&self, values: &ValueMap) -> String;
}
impl<T: Print + Sized> IRDump for T {
    fn ir_dump(&self) -> String { ir_dump(self) }
    // fn ir_dump_with(&self, values: &ValueMap) -> String { ir_dump_with(values, self) }
}

impl Print for String {
    fn print(&self, p: &mut Printer) { p.print_unescaped_str(self); }
}

macro_rules! impl_print_for_number {
    ($($ty:ty),*) => {
        $(
            impl Print for $ty {
                fn print(&self, p: &mut Printer) {
                    write!(p, "{}", self);
                }
            }
        )*
    };
}
impl_print_for_number!(bool, i8, i16, i32, i64, u8, u16, u32, u64, f32, f64, isize, usize);
impl_print_for_number!(BigUint, BigInt);

impl<K: Key,T: Print> Print for SlotMap<K,T> {
    fn print(&self, p: &mut Printer) {
        p.print_list("(", "", ")", true, false, self.iter().map(|(_, v)| v));
    }
}

impl<T: Print> Print for Vec<T> {
    fn print(&self, p: &mut Printer) {
        p.print_list("(", "", ")", false, false, self.iter());
    }
}
impl<T: Print, const N: usize> Print for [T; N] {
    fn print(&self, p: &mut Printer) {
        p.print_list("(", "", ")", true, false, self.iter());
    }
}
impl<T: Print> Print for Option<T> {
    fn print(&self, p: &mut Printer) {
        p.print_list("(", "", ")", true, false, self.iter())
    }
}
impl<T: Print> Print for Box<T> {
    fn print(&self, p: &mut Printer) { self.as_ref().print(p); }
}
impl<T: Print> Print for Range<T> {
    fn print(&self, p: &mut Printer) {
        self.start.print(p);
        write!(p, "..");
        self.end.print(p);
    }
}
