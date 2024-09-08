use std::collections::btree_map::Values;

use syn::{
  AngleBracketedGenericArguments, GenericArgument, GenericParam, Meta,
  MetaNameValue, Path, PathArguments, TypeParam, TypePath,
};

use super::*;

fn lowercasize(s: String) -> String {
  let mut res = String::new();
  let mut is_first = true;
  for c in s.chars() {
    if c.is_uppercase() {
      if !is_first {
        res.push('_');
      }
      is_first = false;
    }
    res.push(c.to_ascii_lowercase());
  }
  res
}

pub fn derive_pp_sexpr_(
  tokens: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
  let DeriveInput {
    ident,
    data,
    attrs,
    vis,
    generics,
  } = syn::parse_macro_input!(tokens);
  let token = quote! {kir::Token};
  let mut surrounded = false;

  match data {
    Data::Struct(DataStruct { fields, .. }) => {
      let info = FieldsInfo::new(Some(ident.clone()), &fields, "pp");
      let pat = info.gen_inner_pat(None);
      let (parse, print, ctx_impl) = impl_op_parse(&info);
      quote! {
          impl kir::Parse for #ident {
              fn parse(parser: &mut kir::Parser) -> Result<Self, String> {
                  #parse
                  Ok(Self #pat)
              }
          }
          impl kir::Print for #ident {
              fn print<'p>(&'p self, p: &mut kir::Printer<'p>) {
                  let Self #pat = self;
                  #print
              }
          }
          impl kir::ParsePrint for #ident {}
          #ctx_impl
      }
      .into()
    }
    Data::Enum(DataEnum { variants, .. }) => {
      let infos = VariantsInfo::new(&variants, "pp");
      let mut parse_matches = vec![];
      let mut print_matches = vec![];
      for info in &infos.infos {
        for (name, value) in &info.args {
          if name == "surrounded" {
            if let Some(value) = value {
              if let Ok(value) =
                syn::parse2::<syn::LitBool>(value.to_token_stream())
              {
                surrounded = value.value;
              }
            } else {
              surrounded = true;
            }
          }
        }
        let name = &info.name;
        let name_lower = lowercasize(info.name.to_string());
        // proc_panic!(name.span().unwrap(), format!("{}", name_lower));
        let (parse, print, ctx_impl) = impl_op_parse(&info.fields);
        if !ctx_impl.is_empty() {
          proc_panic!(
            name.span().unwrap(),
            "ctx_impl for maps is not supported for enum"
          );
        }
        let pat = info.fields.gen_inner_pat(None);
        parse_matches.push(quote! {
            #name_lower => {
                #parse
                Ok(Self::#name #pat)
            }
        });
        print_matches.push(quote! {
            Self::#name #pat => {
                if #surrounded {
                  write!(p, "(");
                }
                write!(p, "{}", #name_lower);
                #print
                if #surrounded {
                  write!(p, ")");
                }
            }
        });
      }
      let expect = infos
        .variants
        .iter()
        .map(|v| lowercasize(v.ident.to_string()))
        .collect::<Vec<_>>();
      let expect = expect.join(", ");
      quote! {
          impl kir::Parse for #ident {
              fn parse(parser: &mut kir::Parser) -> Result<Self, String> {
                  // parse sexpr: (variant args...)
                  if #surrounded {
                    let _ = parser.expect(#token::LParen)?;
                  }
                  let kw = parser.expect(#token::Keyword)?;
                  let res = match kw {
                      #(#parse_matches),*
                      _ => Err(format!("Unknown keyword: {:?} expect {}", kw, #expect)),
                  };
                  if #surrounded {
                    let _ = parser.expect(#token::RParen)?;
                  }
                  res
              }
          }
          impl kir::Print for #ident {
              fn print<'p>(&'p self, p: &mut kir::Printer<'p>) {
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

fn impl_op_parse(info: &FieldsInfo) -> (TokenStream, TokenStream, TokenStream) {
  let mut parse = vec![];
  let mut print = vec![];
  let mut map = None;
  let token = quote! {kir::Token};
  for info in &info.infos {
    let mut parse_before = vec![];
    let mut parse_after = vec![];
    let mut print_before = vec![];
    let mut print_after = vec![];
    let fname = &info.name;
    let mut list_arg = ListArgs::default();
    let mut is_map = false;
    for (name, value) in &info.args {
      match &name[..] {
        "map" => {
          is_map = true;
        }
        "surrounded" => {
          parse_before.push(quote! {parser.expect(#token::LParen)?;});
          print_before.push(quote! {write!(p, "(");});
          if value.is_some() {
            parse_before
              .push(quote! {parser.expect_str(#token::Keyword, #value)?;});
            print_before.push(quote! {write!(p, "{}", #value);});
          }

          parse_after.push(quote! {parser.expect(#token::RParen)?;});
          print_after.push(quote! {write!(p, ")");});
        }
        "open" => {
          parse_before.push(quote! {parser.expect(#token::LParen)?;});
          print_before.push(quote! {write!(p, "(");});
          if value.is_some() {
            print_before.push(quote! {p.ident(#value);});
            print_before.push(quote! {p.newline();});
          }
        }
        "close" => {
          parse_after.push(quote! {parser.expect(#token::RParen)?;});
          if value.is_some() {
            print_after.push(quote! {p.ident(#value);});
            print_after.push(quote! {p.newline();});
          }
          print_after.push(quote! {write!(p, ")");});
        }
        "list_ml" => {
          list_arg = ListArgs {
            left: Some(syn::parse2(quote! {"("}).unwrap()),
            sep: None,
            right: Some(syn::parse2(quote! {")"}).unwrap()),
            newline: Some(syn::parse2(quote! {true}).unwrap()),
            last: None,
          }
        }
        "list" => {
          list_arg = ListArgs {
            left: Some(syn::parse2(quote! {"("}).unwrap()),
            sep: None,
            right: Some(syn::parse2(quote! {")"}).unwrap()),
            newline: Some(syn::parse2(quote! {false}).unwrap()),
            last: None,
            // last: Some(syn::parse2(quote! {true}).unwrap()),
          }
        }
        "nl" => print_before.push(quote! {p.newline();}),
        "nl_" => print_after.push(quote! {p.newline();}),
        // "ident" => print_before.push(quote! {p.ident(#value);}),
        // "ident_" => print_after.push(quote! {p.ident(#value);}),
        name => {
          match name {
            "kw" => parse_before
              .push(quote! {parser.expect_str(#token::Keyword, #value)?;}),
            "kw_" => parse_after
              .push(quote! {parser.expect_str(#token::Keyword, #value)?;}),
            "punct" => parse_before
              .push(quote! {parser.expect_str(#token::Punct, #value)?;}),
            "punct_" => parse_after
              .push(quote! {parser.expect_str(#token::Punct, #value)?;}),
            _ => proc_panic!(value.span().unwrap(), "Unknown attribute"),
          }
          match name {
            "kw" => print_before.push(quote! {write!(p, "{}", #value);}),
            "kw_" => print_after.push(quote! {write!(p, "{}", #value);}),
            "punct" => print_before.push(quote! {write!(p, "{}", #value);}),
            "punct_" => print_after.push(quote! {write!(p, "{}", #value);}),
            _ => proc_panic!(value.span().unwrap(), "Unknown attribute"),
          }
        }
      }
    }
    if is_map {
      map = Some(MapInfo::new(info));
      continue;
    }
    let (parse_expr, print_expr) =
      (list_arg.emit_parse(), list_arg.emit_print(quote! {#fname}));
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
  let ctx_impl = if let Some(MapInfo {
    field_name,
    key_type,
    value_type,
  }) = map.clone()
  {
    if let Some(struct_ident) = info.struct_ident.clone() {
      impl_ctx_map(struct_ident, field_name, key_type, value_type)
    } else {
      TokenStream::new()
    }
  } else {
    TokenStream::new()
  };
  let parse_expr = match &map {
    Some(MapInfo { field_name, .. }) => quote! {
        let __saved_resolver = parser.set_resolver(Some(Default::default()));
        #(#parse)*
        let #field_name = parser.set_resolver(__saved_resolver).unwrap().values;
    },
    None => quote! {#(#parse)*},
  };
  let print_expr = match map {
    Some(MapInfo { field_name, .. }) => quote! {
        let __saved_printer = p.set_printer(Some(kir::ValuePrinter::new(#field_name)));
        #(#print)*
        p.set_printer(__saved_printer);
    },
    None => quote! {#(#print)*},
  };
  (parse_expr, print_expr, ctx_impl)
}


#[derive(Debug, Clone)]
struct MapInfo {
  field_name: Ident,
  key_type: Type,
  value_type: Type,
}

impl MapInfo {
  pub fn new(info: &FieldInfo) -> Self {
    let field_name = info.name.clone();
    let ty = info.field.ty.clone();
    let (key_type, value_type) = if let Type::Path(TypePath {
      path: Path { segments, .. },
      ..
    }) = &ty
    {
      if let Some(segment) = segments.first() {
        if segment.ident == "SlotMap" {
          let args = if let PathArguments::AngleBracketed(
            AngleBracketedGenericArguments { args, .. },
          ) = &segment.arguments
          {
            args
              .iter()
              .filter_map(|arg| {
                if let GenericArgument::Type(ty) = arg {
                  Some(ty.clone())
                } else {
                  None
                }
              })
              .collect::<Vec<_>>()
          } else {
            vec![]
          };

          if args.len() != 2 {
            proc_panic!(
              ty.span().unwrap(),
              &format!(
                "only support SlotMap<K,V> here, but the provided ty is {}",
                ty.to_token_stream().to_string()
              )
            );
          } else {
            (args[0].clone(), args[1].clone())
          }
        } else {
          proc_panic!(
            ty.span().unwrap(),
            &format!(
              "only support SlotMap<K,V> here, but the provided ty is {}",
              ty.to_token_stream().to_string()
            )
          );
        }
      } else {
        proc_panic!(
          ty.span().unwrap(),
          &format!(
            "no segment in type path: {}",
            ty.to_token_stream().to_string()
          )
        );
      }
    } else {
      proc_panic!(
        ty.span().unwrap(),
        &format!(
          "only support SlotMap<K,V> here, but the provided ty is {}",
          ty.to_token_stream().to_string()
        )
      );
    };
    Self {
      field_name,
      key_type,
      value_type,
    }
  }
}


fn impl_ctx_map(
  struct_ident: Ident,
  field_name: Ident,
  key_type: Type,
  value_type: Type,
) -> TokenStream {
  quote! {
    impl kir::Ctx<#value_type, #key_type> for #struct_ident {
      fn insert(&mut self, value: #value_type) -> #key_type {
        kir::Ctx::insert(&mut self.#field_name, value)
      }

      fn insert_with_key(&mut self, f: impl FnOnce(#key_type) -> #value_type) -> #key_type {
        kir::Ctx::insert_with_key(&mut self.#field_name, f)
      }

      fn get(&self, idx: #key_type) -> Option<&#value_type> {
        kir::Ctx::get(&self.#field_name, idx)
      }

      fn get_mut(&mut self, idx: #key_type) -> Option<&mut #value_type> {
        kir::Ctx::get_mut(&mut self.#field_name, idx)
      }

      fn remove(&mut self, idx: #key_type) -> Option<#value_type> {
        kir::Ctx::remove(&mut self.#field_name, idx)
      }

      fn exists(&self, idx: #key_type) -> bool {
        kir::Ctx::exists(&self.#field_name, idx)
      }
    }

    impl std::ops::Index<#key_type> for #struct_ident {
      type Output = #value_type;

      fn index(&self, idx: #key_type) -> &Self::Output {
        self.get(idx).expect("Index not found")
      }
    }

    impl std::ops::IndexMut<#key_type> for #struct_ident {
      fn index_mut(&mut self, idx: #key_type) -> &mut Self::Output {
        self.get_mut(idx).expect("Index not found")
      }
    }
  }
}
