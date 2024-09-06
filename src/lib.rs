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
pub use slotmap::SlotMap;

#[macro_export]
macro_rules! new_key_type {
    ($($vis:vis struct $id:ident; => $val:ident)*) => {
        $(
            #[derive(Clone, Copy, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
            $vis struct $id(slotmap::KeyData);
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

            impl IdFor<$val> for $id {}
        )*
    };
}


pub trait IdFor<T> {}

pub trait Ctx<T, Key>
where
  Key: IdFor<T>,
{
  fn get(&self, key: Key) -> Option<&T>;
  fn get_mut(&mut self, key: Key) -> Option<&mut T>;
  fn remove(&mut self, key: Key) -> Option<T>;
  fn exists(&self, key: Key) -> bool;
  fn insert_with_key(&mut self, value_fn: impl FnOnce(Key) -> T) -> Key;
  fn insert(&mut self, value: T) -> Key;
}

impl<T, Key: slotmap::Key + IdFor<T>> Ctx<T, Key> for SlotMap<Key, T> {
  fn get(&self, key: Key) -> Option<&T> {
      self.get(key)
  }
  fn get_mut(&mut self, key: Key) -> Option<&mut T> {
      self.get_mut(key)
  }
  fn remove(&mut self, key: Key) -> Option<T> {
      self.remove(key)
  }
  fn exists(&self, key: Key) -> bool {
      self.contains_key(key)
  }
  fn insert_with_key(&mut self, value_fn: impl FnOnce(Key) -> T) -> Key {
      self.insert_with_key(value_fn)
  }
  fn insert(&mut self, value: T) -> Key {
      self.insert(value)
  }
}

pub trait KeyIter {
  type Key;
  fn key_vec(&self) -> Vec<Self::Key>;
}

impl<T, Key: slotmap::Key + IdFor<T>> KeyIter for SlotMap<Key, T> {
  type Key = Key;
  fn key_vec(&self) -> Vec<Self::Key> {
    let keys: Vec<Key> = self.iter().map(|(k, _)| k).collect();
    keys
  }
}

pub trait PrintableId {
  fn prefix(&self) -> &'static str;
}
