#![feature(proc_macro_diagnostic)]
#![allow(unused)]

use std::collections::HashMap;

use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{
  parse::Parse, punctuated::Punctuated, spanned::Spanned, Attribute, Data,
  DataEnum, DataStruct, DeriveInput, Expr, ExprLet, ExprLit, Field, Fields,
  FieldsNamed, FieldsUnnamed, Ident, Lit, LitStr, MetaList, PatLit, Token,
  Type, Variant,
};

struct AttrArg {
  name: Ident,
  value: Option<Expr>,
}
impl Parse for AttrArg {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    let name = input.parse()?;
    let value = if input.peek(syn::Token![=]) {
      input.parse::<syn::Token![=]>()?;
      Some(input.parse()?)
    } else {
      None
    };
    Ok(AttrArg { name, value })
  }
}
struct AttrArgList {
  attrs: Punctuated<AttrArg, Token![,]>,
}
impl Parse for AttrArgList {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    let attrs = input.parse_terminated(AttrArg::parse, Token![,])?;
    Ok(AttrArgList { attrs })
  }
}
fn parse_attrs(
  attrs: &[Attribute],
  helper: &str,
) -> Vec<(String, Option<Expr>)> {
  // let mut res: HashMap<String, Option<Expr>> = HashMap::new();
  let mut res = vec![];
  for attr in attrs {
    if let syn::Meta::List(MetaList { path, tokens, .. }) = &attr.meta {
      if let Some(id) = path.get_ident() {
        if id == helper {
          let AttrArgList { attrs } = syn::parse2(tokens.clone()).unwrap();
          for AttrArg { name, value } in attrs {
            res.push((name.to_string(), value));
          }
        }
      }
    }
  }
  res
}

macro_rules! proc_panic {
  ($e:expr, $err:expr) => {{
    proc_macro::Diagnostic::spanned($e, proc_macro::Level::Error, $err).emit();
    panic!("{}", $err);
  }};
}
macro_rules! proc_assert {
  ($cond:expr, $e:expr, $err:expr) => {
    if !$cond {
      proc_panic!($e, $err);
    }
  };
}
macro_rules! unwrap {
  ($span:expr, $e:expr) => {
    match $e {
      Some(e) => e,
      None => proc_panic!($span, "Unexpected None"),
    }
  };
}

#[derive(Clone)]
struct FieldInfo<'r> {
  field: &'r Field,
  name: Ident,
  args: Vec<(String, Option<Expr>)>,
  args_map: HashMap<String, Option<Expr>>,
}

#[derive(Clone, Copy)]
enum FieldsType {
  Named,
  Unnamed,
  Unit,
}
struct FieldsInfo<'r> {
  struct_ident: Option<Ident>,
  span: Span,
  tpe: FieldsType,
  infos: Vec<FieldInfo<'r>>,
}
impl<'r> FieldsInfo<'r> {
  fn new(
    struct_ident: Option<Ident>,
    fields: &'r Fields,
    helper: &str,
  ) -> Self {
    let span = fields.span();
    let (flist, tpe) = match fields {
      Fields::Named(FieldsNamed { named, .. }) => (named, FieldsType::Named),
      Fields::Unnamed(FieldsUnnamed { unnamed, .. }) => {
        (unnamed, FieldsType::Unnamed)
      }
      Fields::Unit => {
        return Self {
          struct_ident,
          span,
          tpe: FieldsType::Unit,
          infos: vec![],
        }
      }
    };
    let mut infos = vec![];
    for (idx, field) in flist.iter().enumerate() {
      let args = parse_attrs(&field.attrs, helper);
      let args_map = args.iter().cloned().collect();
      let name = match &field.ident {
        Some(ident) => ident.clone(),
        None => Ident::new(&format!("_{}", idx), field.ty.span()),
      };
      infos.push(FieldInfo {
        field,
        name,
        args,
        args_map,
      });
    }
    Self {
      struct_ident,
      span,
      tpe,
      infos,
    }
  }
  fn gen_inner_pat(&self, prefix: Option<&str>) -> TokenStream {
    match self.tpe {
      FieldsType::Named => {
        if let Some(prefix) = prefix {
          let mapper = |i: &FieldInfo| {
            let name = &i.name;
            let prefixed =
              Ident::new(&format!("{}{}", prefix, name), name.span());
            quote! {#name : #prefixed}
          };
          let new_inner = self.infos.iter().map(mapper);
          quote! { { #(#new_inner),* } }
        } else {
          let inner = self.infos.iter().map(|i| &i.name);
          quote! { { #(#inner),* } }
        }
      }
      FieldsType::Unnamed => {
        if let Some(prefix) = prefix {
          let new_inner = self.infos.iter().enumerate().map(|(i, _)| {
            let name = Ident::new(&format!("{}{}", prefix, i), self.span);
            quote! { #name }
          });
          quote! { ( #(#new_inner),* ) }
        } else {
          let inner = self.infos.iter().enumerate().map(|(i, _)| {
            let name = Ident::new(&format!("_{}", i), self.span);
            quote! { #name }
          });
          quote! { ( #(#inner),* ) }
        }
      }
      FieldsType::Unit => quote! {},
    }
  }
}

struct VariantInfo<'r> {
  args: Vec<(String, Option<Expr>)>,
  args_map: HashMap<String, Option<Expr>>,
  name: Ident,
  fields: FieldsInfo<'r>,
}
struct VariantsInfo<'r> {
  variants: &'r Punctuated<Variant, Token![,]>,
  infos: Vec<VariantInfo<'r>>,
}
impl<'r> VariantsInfo<'r> {
  fn new(variants: &'r Punctuated<Variant, Token![,]>, helper: &str) -> Self {
    let infos = variants
      .iter()
      .map(|v| {
        let args = parse_attrs(&v.attrs, helper);
        let name = v.ident.clone();
        let fields = FieldsInfo::new(None, &v.fields, helper);
        let args_map = args.iter().cloned().collect();
        VariantInfo {
          args,
          args_map,
          name,
          fields,
        }
      })
      .collect();
    Self { variants, infos }
  }
  fn gen_match(
    &self,
    f: impl Fn(&VariantInfo<'r>) -> TokenStream,
  ) -> TokenStream {
    let match_inner = self.infos.iter().map(|v| {
      let name = &v.name;
      let pat: TokenStream = v.fields.gen_inner_pat(None);
      let content = f(&v);
      quote! {
          Self::#name #pat => {
              #content
          }
      }
    });
    quote! {
        match self {
            #(#match_inner)*
        }
    }
  }
}

mod opio;

#[proc_macro_derive(OpIO, attributes(opio))]
pub fn derive_opio(tokens: proc_macro::TokenStream) -> proc_macro::TokenStream {
  // derive_opio_(tokens)
  match std::panic::catch_unwind(|| opio::derive_opio_(tokens)) {
    Ok(tokens) => tokens,
    Err(_) => proc_macro::TokenStream::new(),
  }
}

#[derive(Default)]
struct ListArgs {
  kw: Option<Expr>,
  left: Option<Expr>,
  sep: Option<Expr>,
  right: Option<Expr>,
  newline: Option<Expr>,
  last: Option<Expr>,
}
impl ListArgs {
  fn any(&self) -> bool {
    self.left.is_some()
      || self.sep.is_some()
      || self.right.is_some()
      || self.newline.is_some()
      || self.last.is_some()
      || self.kw.is_some()
  }
  fn kw(&self) -> Option<TokenStream> {
    self.kw.as_ref().map(|e| quote! {#e})
  }
  fn left(&self) -> TokenStream {
    self.left.as_ref().map_or(quote! {""}, |e| quote! {#e})
  }
  fn sep(&self) -> TokenStream {
    self.sep.as_ref().map_or(quote! {""}, |e| quote! {#e})
  }
  fn right(&self) -> TokenStream {
    self.right.as_ref().map_or(quote! {""}, |e| quote! {#e})
  }
  fn newline(&self) -> TokenStream {
    self
      .newline
      .as_ref()
      .map_or(quote! {false}, |e| quote! {#e})
  }
  fn last(&self) -> TokenStream {
    self.last.as_ref().map_or(quote! {false}, |e| quote! {#e})
  }
  fn emit_parse(&self) -> TokenStream {
    let kw = self.kw();
    let sep = self.sep();
    if self.any() {
      if let Some(kw) = self.kw() {
        quote! {
          parser.parse_list_kw(Some(#kw), #sep)
        }
      } else {
        quote! {
          parser.parse_list(#sep)
        }
      }
    } else {
      quote! {
          parser.parse()
      }
    }
  }
  fn emit_print(&self, value: impl ToTokens) -> TokenStream {
    let kw = self.kw().unwrap_or(quote! {""});
    let left = self.left();
    let sep = self.sep();
    let right = self.right();
    let newline = self.newline();
    let last = self.last();
    if self.any() {
      quote! {
          p.print_list(#kw, #left, #sep, #right, #newline, #last, #value);
      }
    } else {
      quote! {
          p.print(#value)
      }
    }
  }
}

mod parse_print;

#[proc_macro_derive(ParsePrint, attributes(pp))]
pub fn derive_parse_print(
  tokens: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
  match std::panic::catch_unwind(|| parse_print::derive_parse_print_(tokens)) {
    Ok(tokens) => tokens,
    Err(_) => proc_macro::TokenStream::new(),
  }
}

mod sexpr;

#[proc_macro_derive(SExpr, attributes(pp))]
pub fn derive_pp_sexpr(
  tokens: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
  match std::panic::catch_unwind(|| sexpr::derive_pp_sexpr_(tokens)) {
    Ok(tokens) => tokens,
    Err(_) => proc_macro::TokenStream::new(),
  }
}
