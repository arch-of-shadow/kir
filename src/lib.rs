#![feature(trait_upcasting)]

pub mod flatindex;

mod examples;
mod ir;
mod parse;
mod print;

pub use ir::*;
pub use parse::*;
pub use print::*;

pub trait ParsePrint: parse::Parse + print::Print {}

pub use kir_macros::{OpIO, ParsePrint, SExpr};
pub use std::any::Any;

pub trait HasStaticName {
  fn name() -> &'static str;
}

pub trait HasResolver {
  type Resolver: Resolver;
  fn name(&self) -> String;
}

#[macro_export]
macro_rules! new_key_type {
    // "value" => pub struct ValueId;
    ($($name:literal => $vis:vis struct $id:ident;)*) => {
        $(
            #[derive(Clone, Copy, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
            $vis struct $id(slotmap::KeyData);
            impl kir::HasStaticName for $id {
              fn name() -> &'static str { $name }
            }
            impl std::fmt::Display for $id {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    write!(f, "{}", self.0.as_ffi() as u32)
                }
            }
            impl std::fmt::Debug for $id {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    self.0.fmt(f)
                }
            }
            impl From<slotmap::KeyData> for $id {
                fn from(data: slotmap::KeyData) -> Self { Self(data) }
            }
            unsafe impl slotmap::Key for $id {
                fn data(&self) -> slotmap::KeyData { self.0 }
            }
        )*
    };
}
