#![feature(proc_macro_diagnostic)]
#![allow(unused)]

use std::collections::HashMap;

use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{
  parse::Parse, punctuated::Punctuated, spanned::Spanned, Attribute, Data, DataEnum, DataStruct,
  DeriveInput, Expr, ExprLet, ExprLit, Field, Fields, FieldsNamed, FieldsUnnamed, Ident, Lit,
  LitStr, MetaList, PatLit, Token, Type, Variant,
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
fn parse_attrs(attrs: &[Attribute], helper: &str) -> Vec<(String, Option<Expr>)> {
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
  span: Span,
  tpe: FieldsType,
  infos: Vec<FieldInfo<'r>>,
}
impl<'r> FieldsInfo<'r> {
  fn new(fields: &'r Fields, helper: &str) -> Self {
    let span = fields.span();
    let (flist, tpe) = match fields {
      Fields::Named(FieldsNamed { named, .. }) => (named, FieldsType::Named),
      Fields::Unnamed(FieldsUnnamed { unnamed, .. }) => (unnamed, FieldsType::Unnamed),
      Fields::Unit => {
        return Self {
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
    Self { span, tpe, infos }
  }
  fn gen_inner_pat(&self, prefix: Option<&str>) -> TokenStream {
    match self.tpe {
      FieldsType::Named => {
        if let Some(prefix) = prefix {
          let mapper = |i: &FieldInfo| {
            let name = &i.name;
            let prefixed = Ident::new(&format!("{}{}", prefix, name), name.span());
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
  // fn filtered(&self, pred: impl Fn(&FieldInfo) -> bool) -> Self {
  //     Self {
  //         span: self.span,
  //         tpe: self.tpe,
  //         infos: self.infos.iter().filter(|i| pred(i)).cloned().collect(),
  //     }
  // }
  // fn gen_decl<R: ToTokens>(&self, type_mapper: impl Fn(&Type) -> R) -> TokenStream {
  //     match self.tpe {
  //         FieldsType::Named => {
  //             let inner = self.infos.iter().map(|i| {
  //                 let name = &i.name;
  //                 let ty = type_mapper(&i.field.ty);
  //                 quote! { #name : #ty }
  //             });
  //             quote! { { #(#inner),* } }
  //         }
  //         FieldsType::Unnamed => {
  //             let inner = self.infos.iter().map(|i| type_mapper(&i.field.ty));
  //             quote! { ( #(#inner),* ); }
  //         }
  //         FieldsType::Unit => quote! {;},
  //     }
  // }
  // fn gen_inst(&self, args: Vec<impl ToTokens>) -> TokenStream {
  //     match self.tpe {
  //         FieldsType::Named => {
  //             let inner = self.infos.iter().zip(args).map(|(i, a)| {
  //                 let name = &i.name;
  //                 let ty = &i.field.ty;
  //                 quote! { #name : #a }
  //             });
  //             quote! { { #(#inner),* } }
  //         }
  //         FieldsType::Unnamed => {
  //             let inner = self.infos.iter().zip(args).map(|(i, a)| {
  //                 let ty = &i.field.ty;
  //                 quote! { #a }
  //             });
  //             quote! { ( #(#inner),* ) }
  //         }
  //         FieldsType::Unit => quote! {},
  //     }
  // }
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
        let fields = FieldsInfo::new(&v.fields, helper);
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
  fn gen_match(&self, f: impl Fn(&VariantInfo<'r>) -> TokenStream) -> TokenStream {
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

// fn impl_opio_func(info: &FieldsInfo, name: &str, gen_mut: bool, wrap_view: Ident) -> (TokenStream, TokenStream) {
//     let mut unpacks = vec![];
//     let mut uses = vec![];
//     for item in &info.infos {
//         let ident = &item.name;
//         if item.args_map.contains_key(name) {
//             if gen_mut {
//                 unpacks.push(quote!{let #ident = &mut self.#ident;});
//             } else {
//                 unpacks.push(quote!{let #ident = &self.#ident;});
//             }
//             uses.push(quote!{#ident});
//         }
//         else if item.args_map.contains_key("wrap") {
//             if gen_mut {
//                 unpacks.push(quote!{let mut #ident = kir::#wrap_view(&mut self.#ident);});
//                 uses.push(quote!{&mut #ident});
//             } else {
//                 unpacks.push(quote!{let #ident = kir::#wrap_view(&self.#ident);});
//                 uses.push(quote!{&#ident});
//             }
//         }
//     }
//     (quote!{#(#unpacks)*}, quote!{#(#uses),*})
// }

fn derive_opio_(tokens: proc_macro::TokenStream) -> proc_macro::TokenStream {
  let DeriveInput {
    ident,
    data,
    attrs,
    vis,
    generics,
  } = syn::parse_macro_input!(tokens);
  let value_id = quote! {kir::ValueId};
  match data {
    Data::Struct(DataStruct { fields, .. }) => {
      let info = FieldsInfo::new(&fields, "opio");
      let keys = ["input", "output", "attr", "wrap"];
      for item in &info.infos {
        let traits = item.args_map.contains_key("input") as u8
          + item.args_map.contains_key("output") as u8
          + item.args_map.contains_key("attr") as u8
          + item.args_map.contains_key("wrap") as u8;
        if traits > 1 {
          proc_panic!(
            item.field.span().unwrap(),
            "Only one of input, output, attr and wrap is allowed"
          );
        }
      }
      let find_all = |name: &str, use_mut: bool, wrap_len: &str, wrap_get: &str| {
        let mut res = vec![];
        let wrap_len = Ident::new(wrap_len, Span::call_site());
        let wrap_get = Ident::new(wrap_get, Span::call_site());
        for info in &info.infos {
          let iname = &info.name;
          if info.args_map.contains_key(name) {
            if use_mut {
              res.push(quote! { &mut self.#iname });
            } else {
              res.push(quote! { &self.#iname });
            }
          } else if info.args_map.contains_key("wrap") {
            if use_mut {
              res.push(quote! { <&mut self.#iname, #wrap_len, #wrap_get>})
            } else {
              res.push(quote! { <&self.#iname, #wrap_len, #wrap_get> });
            }
          }
        }
        quote! { #(#res),* }
      };
      // let pat = info.gen_inner_pat(None);
      // let cmp_pat = info.gen_inner_pat(Some("_r"));
      // let (input_upk, inputs) = impl_opio_func(&info, "input", false, Ident::new("InputView", Span::call_site()));
      // let (input_mut_upk, inputs_mut) = impl_opio_func(&info, "input", true, Ident::new("InputViewMut", Span::call_site()));
      // let (output_upk, outputs) = impl_opio_func(&info, "output", false, Ident::new("OutputView", Span::call_site()));
      // let (output_mut_upk, outputs_mut) = impl_opio_func(&info, "output", true, Ident::new("OutputViewMut", Span::call_site()));
      // let (attr_upk, attrs) = impl_opio_func(&info, "attr", false, Ident::new("AttrView", Span::call_site()));
      let inputs = find_all("input", false, "num_inputs", "input");
      let inputs_mut = find_all("input", true, "num_inputs", "input_mut");
      let outputs = find_all("output", false, "num_outputs", "output");
      let outputs_mut = find_all("output", true, "num_outputs", "output_mut");
      // let outputs = find_all("output");
      let find_ident = |name: &str| {
        info
          .infos
          .iter()
          .filter(|i| i.args_map.contains_key(name))
          .map(|i| &i.name)
          .collect::<Vec<_>>()
      };
      let attrs = find_ident("attr");
      let wraps = find_ident("wrap");
      quote! {
          impl kir::OpIO for #ident {
              fn num_inputs(&self) -> usize {
                  // #input_upk
                  // kir::flat_combine_len!{<#value_id> #inputs}
                  kir::flat_combine_len!{<#value_id> #inputs}
              }
              fn input(&self, i: usize) -> #value_id {
                  // #input_upk
                  // kir::flat_combine_deref!{<#value_id> i, #inputs}
                  kir::flat_combine_deref!{<#value_id> i, #inputs}
              }
              fn input_mut(&mut self, i: usize) -> &mut #value_id {
                  // #input_mut_upk
                  // kir::flat_combine_mut!{<#value_id> i, #inputs_mut}
                  kir::flat_combine_mut!{<#value_id> i, #inputs_mut}
              }
              fn num_outputs(&self) -> usize {
                  // #output_upk
                  // kir::flat_combine_len!{<#value_id> #outputs}
                  kir::flat_combine_len!{<#value_id> #outputs}
              }
              fn output(&self, i: usize) -> #value_id {
                  // #output_upk
                  // kir::flat_combine_deref!{<#value_id> i, #outputs}
                  kir::flat_combine_deref!{<#value_id> i, #outputs}
              }
              fn output_mut(&mut self, i: usize) -> &mut #value_id {
                  // #output_mut_upk
                  // kir::flat_combine_mut!{<#value_id> i, #outputs_mut}
                  kir::flat_combine_mut!{<#value_id> i, #outputs_mut}
              }
              fn attr_eq(&self, rhs: &Self) -> bool {
                  #(
                      if !std::cmp::PartialEq::eq(&self.#attrs, &rhs.#attrs) {
                          return false;
                      }
                  )*
                  #(
                      if !self.#wraps.attr_eq(&rhs.#wraps) {
                          return false;
                      }
                  )*
                  true
              }
              fn attr_hash<H: std::hash::Hasher>(&self, state: &mut H) {
                  #(
                      std::hash::Hash::hash(&self.#attrs, state);
                  )*
                  #(
                      self.#wraps.attr_hash(state);
                  )*
              }
          }
      }
      .into()
    }
    Data::Enum(DataEnum { variants, .. }) => {
      let info = VariantsInfo::new(&variants, "opio");
      for item in &info.infos {
        if item.fields.infos.len() != 1 {
          proc_panic!(
            item.fields.span.unwrap(),
            "Only one field is allowed in each variant"
          );
        }
      }
      let num_inputs = info.gen_match(|_| quote! {_0.num_inputs()});
      let num_outputs = info.gen_match(|_| quote! {_0.num_outputs()});
      let input = info.gen_match(|_| quote! {_0.input(i)});
      let input_mut = info.gen_match(|_| quote! {_0.input_mut(i)});
      let output = info.gen_match(|_| quote! {_0.output(i)});
      let output_mut = info.gen_match(|_| quote! {_0.output_mut(i)});
      let attr_eq = info.infos.iter().map(|v| {
        let name = &v.name;
        let pat = v.fields.gen_inner_pat(None);
        let rpat = v.fields.gen_inner_pat(Some("_r"));
        quote! {
            (Self::#name #pat, Self::#name #rpat) => {
                _0.attr_eq(_r0)
            }
        }
      });
      let attr_hash = info.gen_match(|_| quote! {_0.attr_hash(state)});
      quote! {
          impl kir::OpIO for #ident {
              fn num_inputs(&self) -> usize {
                  #num_inputs
              }
              fn input(&self, i: usize) -> #value_id {
                  #input
              }
              fn input_mut(&mut self, i: usize) -> &mut #value_id {
                  #input_mut
              }
              fn num_outputs(&self) -> usize {
                  #num_outputs
              }
              fn output(&self, i: usize) -> #value_id {
                  #output
              }
              fn output_mut(&mut self, i: usize) -> &mut #value_id {
                  #output_mut
              }
              fn attr_eq(&self, rhs: &Self) -> bool {
                  match (self, rhs) {
                      #(#attr_eq)*
                      _ => false
                  }
              }
              fn attr_hash<H: std::hash::Hasher>(&self, state: &mut H) {
                  std::hash::Hash::hash(&std::mem::discriminant(self), state);
                  #attr_hash
              }
          }
      }
      .into()
    }
    _ => {
      proc_panic!(ident.span().unwrap(), "Only struct is supported");
      panic!("Only struct is supported");
    }
  }
}

#[proc_macro_derive(OpIO, attributes(opio))]
pub fn derive_opio(tokens: proc_macro::TokenStream) -> proc_macro::TokenStream {
  // derive_opio_(tokens)
  match std::panic::catch_unwind(|| derive_opio_(tokens)) {
    Ok(tokens) => tokens,
    Err(_) => proc_macro::TokenStream::new(),
  }
}

// fn get_ident(expr: &Expr) -> Ident {
//     match expr {
//         Expr::Path(path) => path.path.get_ident().unwrap().clone(),
//         _ => proc_panic!(expr.span().unwrap(), "Only path is supported"),
//     }
// }

struct ListArgs {
  ty: Type,
  left: Option<Expr>,
  sep: Option<Expr>,
  right: Option<Expr>,
  newline: Option<Expr>,
  last: Option<Expr>,
}


impl ListArgs {
  fn new(ty: Type) -> Self {
    Self {
      ty,
      left: None,
      sep: None,
      right: None,
      newline: None,
      last: None,
    }
  }
  fn any(&self) -> bool {
    self.left.is_some()
      || self.sep.is_some()
      || self.right.is_some()
      || self.newline.is_some()
      || self.last.is_some()
  }
  fn left(&self) -> TokenStream { self.left.as_ref().map_or(quote! {""}, |e| quote! {#e}) }
  fn sep(&self) -> TokenStream { self.sep.as_ref().map_or(quote! {""}, |e| quote! {#e}) }
  fn right(&self) -> TokenStream { self.right.as_ref().map_or(quote! {""}, |e| quote! {#e}) }
  fn newline(&self) -> TokenStream {
    self
      .newline
      .as_ref()
      .map_or(quote! {false}, |e| quote! {#e})
  }
  fn last(&self) -> TokenStream { self.last.as_ref().map_or(quote! {false}, |e| quote! {#e}) }

  fn emit_ty(&self) -> TokenStream {
    self.ty.clone().to_token_stream()
  }
  fn emit_parse(&self) -> TokenStream {
    let sep = self.sep();
    if self.any() {
      quote! {
          parser.parse_list(#sep)
      }
    } else {
      quote! {
          parser.parse()
      }
    }
  }
  fn emit_print(&self, value: impl ToTokens) -> TokenStream {
    let left = self.left();
    let sep = self.sep();
    let right = self.right();
    let newline = self.newline();
    let last = self.last();
    if self.any() {
      quote! {
          p.print_list(#left, #sep, #right, #newline, #last, #value);
      }
    } else {
      quote! {
          p.print(#value)
      }
    }
  }
}

fn impl_op_parse(info: &FieldsInfo) -> (TokenStream, TokenStream) {
  let mut parse = vec![];
  let mut print = vec![];
  let mut value_map = None;
  let token = quote! {kir::Token};
  for info in &info.infos {
    let mut parse_before = vec![];
    let mut parse_after = vec![];
    let mut print_before = vec![];
    let mut print_after = vec![];
    let fname = &info.name;
    let mut list_arg = ListArgs::new(info.field.ty.clone());
    let mut is_value_map = false;
    for (name, value) in &info.args {
      match &name[..] {
        "left" => list_arg.left = Some(value.clone().unwrap()),
        "sep" => list_arg.sep = Some(value.clone().unwrap()),
        "right" => list_arg.right = Some(value.clone().unwrap()),
        "newline" => list_arg.newline = Some(value.clone().unwrap()),
        "last" => list_arg.last = Some(value.clone().unwrap()),
        "value_map" => is_value_map = true,
        "semi_list" => {
          list_arg = ListArgs {
            ty: info.field.ty.clone(),
            left: Some(syn::parse2(quote! {"{"}).unwrap()),
            sep: Some(syn::parse2(quote! {";"}).unwrap()),
            right: Some(syn::parse2(quote! {"}"}).unwrap()),
            newline: Some(syn::parse2(quote! {true}).unwrap()),
            last: Some(syn::parse2(quote! {true}).unwrap()),
          }
        }
        "nl" => print_before.push(quote! {p.newline();}),
        "nl_" => print_after.push(quote! {p.newline();}),
        "ident" => print_before.push(quote! {p.ident(#value);}),
        "ident_" => print_after.push(quote! {p.ident(#value);}),
        name => {
          match name {
            "kw" => parse_before.push(quote! {parser.expect_str(#token::Keyword, #value)?;}),
            "punct" => parse_before.push(quote! {parser.expect_str(#token::Punct, #value)?;}),
            "kw_" => parse_after.push(quote! {parser.expect_str(#token::Keyword, #value)?;}),
            "punct_" => parse_after.push(quote! {parser.expect_str(#token::Punct, #value)?;}),
            _ => proc_panic!(value.span().unwrap(), "Unknown attribute"),
          }
          match name {
            "kw" => print_before.push(quote! {write!(p, "{}", #value);}),
            "punct" => print_before.push(quote! {write!(p, "{}", #value);}),
            "kw_" => print_after.push(quote! {write!(p, "{}", #value);}),
            "punct_" => print_after.push(quote! {write!(p, "{}", #value);}),
            _ => proc_panic!(value.span().unwrap(), "Unknown attribute"),
          }
        }
      }
    }
    if is_value_map {
      value_map = Some(fname.clone());
      continue;
    }
    let (parse_expr, print_expr) = (list_arg.emit_parse(), list_arg.emit_print(quote! {#fname}));
    parse.push(quote! {
        let #fname = {
            #(#parse_before)*
            let __tmp = #parse_expr ?;
            #(#parse_after)*
            __tmp
        };
    });
    print.push(quote! {
        {
            #(#print_before)*
            #print_expr;
            #(#print_after)*
        }
    });
  }
  let builder = info.gen_inner_pat(None);
  let parse_expr = match &value_map {
    Some(vmap) => quote! {
        let __saved_resolver = parser.set_resolver(Some(Default::default()));
        #(#parse)*
        let #vmap = parser.set_resolver(__saved_resolver).unwrap().values;
    },
    None => quote! {#(#parse)*},
  };
  let print_expr = match value_map {
    Some(vmap) => quote! {
        let __saved_printer = p.set_printer(Some(kir::ValuePrinter::new(#vmap)));
        #(#print)*
        p.set_printer(__saved_printer);
    },
    None => quote! {#(#print)*},
  };
  (parse_expr, print_expr)
}

fn derive_parse_print_(tokens: proc_macro::TokenStream) -> proc_macro::TokenStream {
  let DeriveInput {
    ident,
    data,
    attrs,
    vis,
    generics,
  } = syn::parse_macro_input!(tokens);
  let token = quote! {kir::Token};
  match data {
    Data::Struct(DataStruct { fields, .. }) => {
      let info = FieldsInfo::new(&fields, "pp");
      let pat = info.gen_inner_pat(None);
      let (parse, print) = impl_op_parse(&info);
      quote! {
          impl kir::Parse for #ident {
              fn parse(parser: &mut kir::Parser) -> Result<Self, String> {
                  #parse
                  Ok(Self #pat)
              }
          }
          impl kir::Print for #ident {
              fn print(& self, p: &mut kir::Printer) {
                  let Self #pat = self;
                  #print
              }
          }
          impl kir::ParsePrint for #ident {}
      }
      .into()
    }
    Data::Enum(DataEnum { variants, .. }) => {
      let infos = VariantsInfo::new(&variants, "pp");
      let mut parse_matches = vec![];
      let mut print_matches = vec![];
      for info in &infos.infos {
        let name = &info.name;
        let name_lower = info.name.to_string().to_lowercase();
        let (parse, print) = impl_op_parse(&info.fields);
        let pat = info.fields.gen_inner_pat(None);
        parse_matches.push(quote! {
            #name_lower => {
                #parse
                Ok(Self::#name #pat)
            }
        });
        print_matches.push(quote! {
            Self::#name #pat => {
                write!(p, "{}", #name_lower);
                #print
            }
        });
      }
      let expect = infos
        .variants
        .iter()
        .map(|v| v.ident.to_string().to_lowercase())
        .collect::<Vec<_>>();
      let expect = expect.join(", ");
      quote! {
          impl kir::Parse for #ident {
              fn parse(parser: &mut kir::Parser) -> Result<Self, String> {
                  let kw = parser.expect(#token::Keyword)?;
                  match kw {
                      #(#parse_matches),*
                      _ => Err(format!("Unknown keyword: {:?} expect {}", kw, #expect)),
                  }
              }
          }
          impl kir::Print for #ident {
              fn print(& self, p: &mut kir::Printer) {
                  match self {
                      #(#print_matches),*
                  }
              }
          }
          impl kir::ParsePrint for #ident {}
      }
      .into()
    }
    _ => {
      proc_panic!(ident.span().unwrap(), "Only struct and enum are supported");
      panic!("Only struct and enum are supported");
    }
  }
}

#[proc_macro_derive(ParsePrint, attributes(pp))]
pub fn derive_parse_print(tokens: proc_macro::TokenStream) -> proc_macro::TokenStream {
  match std::panic::catch_unwind(|| derive_parse_print_(tokens)) {
    Ok(tokens) => tokens,
    Err(_) => proc_macro::TokenStream::new(),
  }
}

mod sexpr;

#[proc_macro_derive(SExpr, attributes(pp))]
pub fn derive_pp_sexpr(tokens: proc_macro::TokenStream) -> proc_macro::TokenStream {
  match std::panic::catch_unwind(|| sexpr::derive_pp_sexpr_(tokens)) {
    Ok(tokens) => {
      // println!("tokens: {:?}", tokens);
      tokens
    },
    Err(_) => proc_macro::TokenStream::new(),
  }
}
