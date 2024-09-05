use std::collections::btree_map::Values;

use syn::{Meta, MetaNameValue};

use super::*;

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
  for Attribute {
    pound_token,
    style,
    bracket_token,
    meta,
  } in &attrs
  {
    if let Meta::NameValue(MetaNameValue {
      path,
      eq_token,
      value,
      ..
    }) = meta
    {
      if path.is_ident("surrounded") {
        if let Ok(value) = syn::parse2::<syn::LitBool>(value.to_token_stream())
        {
          surrounded = value.value;
        }
      }
    }
  }
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
              fn print<'p>(&'p self, p: &mut kir::Printer<'p>) {
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

        for (name, value) in &info.args {
          if name == "surrounded" {
            if let Some(value) = value {
              if let Ok(value) = syn::parse2::<syn::LitBool>(value.to_token_stream())
              {
                surrounded = value.value;
              }
            } else {
              surrounded = true;
            }
          }
        }
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
        .map(|v| v.ident.to_string().to_lowercase())
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
    let mut list_arg = ListArgs::default();
    let mut is_value_map = false;
    for (name, value) in &info.args {
      match &name[..] {
        "value_map" => is_value_map = true,
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
    if is_value_map {
      value_map = Some(fname.clone());
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
