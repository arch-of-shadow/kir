use std::{
    fmt::Debug,
    ops::{Deref, DerefMut},
    str::FromStr,
};

use crate::new_key_type;
use slotmap::SlotMap;

new_key_type! {
    pub struct ValueId;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Type {
    Int(u32),
    Ref(u32),
    Arr(u32, u32),
}
impl Type {
    pub fn new_unit() -> Type { Type::Int(0) }
    pub fn new_int(width: u32) -> Type { Type::Int(width) }
    pub fn new_ref(width: u32) -> Type { Type::Ref(width) }
    pub fn new_arr(elem_width: u32, depth: u32) -> Type { Type::Arr(elem_width, depth) }
    pub fn int_width(&self) -> u32 {
        match self {
            Type::Int(width) => *width,
            _ => panic!("Type {self:?} is not an integer"),
        }
    }
    pub fn int_to_ref(&self) -> Self {
        match self {
            Type::Int(width) => Type::Ref(*width),
            _ => panic!("Type {self:?} is not an integer"),
        }
    }
    pub fn ref_to_int(&self) -> Self {
        match self {
            Type::Ref(width) => Type::Int(*width),
            _ => panic!("Type {self:?} is not a reference"),
        }
    }
    pub fn ref_width(&self) -> u32 {
        match self {
            Type::Ref(width) => *width,
            _ => panic!("Type is not a reference"),
        }
    }
    pub fn arr_elem_width(&self) -> u32 {
        match self {
            Type::Arr(elem_width, _) => *elem_width,
            _ => panic!("Type is not an array"),
        }
    }
    pub fn arr_depth(&self) -> u32 {
        match self {
            Type::Arr(_, elem_count) => *elem_count,
            _ => panic!("Type is not an array"),
        }
    }
}
impl ToString for Type {
    fn to_string(&self) -> String {
        match self {
            Type::Int(width) => format!("i{}", width),
            Type::Ref(width) => format!("r{}", width),
            Type::Arr(elem_width, depth) => format!("i{}x{}", elem_width, depth),
        }
    }
}
impl FromStr for Type {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // int type: i32, i64, ...
        // arr type: i32x2, i64x8 ...
        let mut parts = s.split('x');
        let width_part = parts
            .next()
            .ok_or_else(|| "Empty type string".to_string())?;
        let width = width_part[1..]
            .parse()
            .map_err(|e| format!("Invalid width: {}", e))?;
        if let Some(depth) = parts.next() {
            let depth = depth.parse().map_err(|e| format!("Invalid depth: {}", e))?;
            Ok(Type::Arr(width, depth))
        } else {
            if width_part.starts_with('i') {
                Ok(Type::Int(width))
            } else if width_part.starts_with('r') {
                Ok(Type::Ref(width))
            } else {
                Err(format!("Invalid type string: {}", s))
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Value {
    pub ty: Type,
    pub name: Option<String>,
}

impl ValueId {
    pub fn ty(&self, t: &SlotMap<ValueId, Value>) -> Type { t[*self].ty }
    pub fn name<'r>(&self, t: &'r SlotMap<ValueId, Value>) -> &'r Option<String> { &t[*self].name }
    pub fn name_mut<'r>(&self, t: &'r mut SlotMap<ValueId, Value>) -> &'r mut Option<String> {
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
    fn values(&self) -> impl Iterator<Item = ValueId> + '_ { self.inputs().chain(self.outputs()) }
    fn map_values(&mut self, mut f: impl FnMut(ValueId) -> ValueId) {
        self.map_inputs(&mut f);
        self.map_outputs(&mut f);
    }
    fn attr_eq(&self, rhs: &Self) -> bool;
    fn attr_hash<H: std::hash::Hasher>(&self, state: &mut H);
}

impl<T: OpIO> OpIO for Box<T> {
    fn num_inputs(&self) -> usize { self.deref().num_inputs() }
    fn input(&self, i: usize) -> ValueId { self.deref().input(i) }
    fn input_mut(&mut self, i: usize) -> &mut ValueId { self.deref_mut().input_mut(i) }
    fn num_outputs(&self) -> usize { self.deref().num_outputs() }
    fn output(&self, i: usize) -> ValueId { self.deref().output(i) }
    fn output_mut(&mut self, i: usize) -> &mut ValueId { self.deref_mut().output_mut(i) }
    fn attr_eq(&self, rhs: &Self) -> bool { self.deref().attr_eq(rhs.deref()) }
    fn attr_hash<H: std::hash::Hasher>(&self, state: &mut H) { self.deref().attr_hash(state) }
}

#[derive(Debug, Clone, Copy)]
pub struct AttrView<'op, T: OpIO>(pub &'op T);
impl<'op, T: OpIO> AttrView<'op, T> {
    pub fn new(op: &'op T) -> Self { Self(op) }
}
impl<'op, T: OpIO> std::ops::Deref for AttrView<'op, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target { self.0 }
}
impl<'op, T: OpIO> std::cmp::PartialEq for AttrView<'op, T> {
    fn eq(&self, rhs: &Self) -> bool { self.0.attr_eq(rhs.0) }
}
impl<'op, T: OpIO> std::cmp::Eq for AttrView<'op, T> {}
impl<'op, T: OpIO> std::hash::Hash for AttrView<'op, T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) { self.0.attr_hash(state) }
}
