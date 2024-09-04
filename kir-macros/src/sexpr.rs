use std::sync::mpsc::RecvError;

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
                write!(p, "(");
                write!(p, "{}", #name_lower);
                #print
                write!(p, ")");
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
                  let _ = parser.expect(#token::LParen)?;
                  let kw = parser.expect(#token::Keyword)?;
                  let res = match kw {
                      #(#parse_matches),*
                      _ => Err(format!("Unknown keyword: {:?} expect {}", kw, #expect)),
                  };
                  let _ = parser.expect(#token::RParen)?;
                  res
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

fn impl_op_parse(info: &FieldsInfo) -> (TokenStream, TokenStream) {
  let mut parse = vec![];
  let mut print = vec![];
  // let mut value_map = None;
  let mut maps = vec![];
  let token = quote! {kir::Token};
  for info in &info.infos {
    let mut parse_before = vec![];
    let mut parse_after = vec![];
    let mut print_before = vec![];
    let mut print_after = vec![];
    let fname = &info.name;
    let mut list_arg = ListArgs::new(info.field.ty.clone());
    let mut is_map = false;
    let mut skip = false;
    let mut resolver = None;
    let mut dumper = None;

    for (name, value) in &info.args {
      match &name[..] {
        "skip" => skip = true,
        "map" => is_map = true,
        "resolver" => resolver = value.clone(),
        "dumper" => dumper = value.clone(),
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
            ty: info.field.ty.clone(),
            left: Some(syn::parse2(quote! {"("}).unwrap()),
            sep: None,
            right: Some(syn::parse2(quote! {")"}).unwrap()),
            newline: Some(syn::parse2(quote! {true}).unwrap()),
            last: None,
          }
        }
        "list" => {
          list_arg = ListArgs {
            ty: info.field.ty.clone(),
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
            // "punct" => parse_before.push(quote! {parser.expect_str(#token::Punct, #value)?;}),
            // "punct_" => parse_after.push(quote! {parser.expect_str(#token::Punct, #value)?;}),
            _ => proc_panic!(value.span().unwrap(), "Unknown attribute"),
          }
          match name {
            "kw" => print_before.push(quote! {write!(p, "{}", #value);}),
            "kw_" => print_after.push(quote! {write!(p, "{}", #value);}),
            // "punct" => print_before.push(quote! {write!(p, "{}", #value);}),
            // "punct_" => print_after.push(quote! {write!(p, "{}", #value);}),
            _ => proc_panic!(value.span().unwrap(), "Unknown attribute"),
          }
        }
      }
    }
    if is_map {
      if let (Some(resolver), Some(dumper)) = (resolver, dumper) {
        maps.push((fname.clone(), resolver, dumper));
      } else {
        proc_panic!(
          info.field.span().unwrap(),
          "map without resolver/printer name"
        );
      }
    }
    if skip {
      continue;
    }

    let (field_ty, parse_expr, print_expr) =
      (list_arg.emit_ty(), list_arg.emit_parse(), list_arg.emit_print(quote! {#fname}));
    parse.push(quote! {
        let #fname = {
            #(#parse_before)*
            let __tmp: #field_ty = #parse_expr ?;
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

  let mut before_parse = vec![];
  let mut after_parse = vec![];
  let mut before_print = vec![];
  let mut after_print = vec![];
  for (fname, resolver, dumper) in &maps {
    // let saved_resolver_string = format!("__saved_resolver_{}", fname.to_string());
    // let saved_resolver_ident: syn::Ident = syn::parse_str(&saved_resolver_string).unwrap();
    let resolver_name_string = format!("__name_resolver_{}", fname.to_string());
    let resolver_name_ident: syn::Ident =
      syn::parse_str(&resolver_name_string).unwrap();
    let dumper_name_string = format!("__name_dumper_{}", fname.to_string());
    let dumper_name_ident: syn::Ident =
      syn::parse_str(&dumper_name_string).unwrap();
    before_parse.push(quote! {
      let __resolver = #resolver::default();
      let #resolver_name_ident = __resolver.name();
      parser.add_resolver(#resolver_name_ident, Box::new(__resolver) as Box<dyn kir::Resolver>);
    });
    after_parse.push(quote! {
      let #fname = parser.pop_resolver::<#resolver>(#resolver_name_ident).map;
    });
    before_print.push(quote! {
      let __dumper = #dumper::from(#fname.clone());
      let #dumper_name_ident = __dumper.name();
      p.add_dumper(#dumper_name_ident, Box::new(__dumper) as Box<dyn Any>);
    });
    after_print.push(quote! {});
  }
  let parse_expr = quote! {
    #(#before_parse)*
    #(#parse)*
    #(#after_parse)*
  };
  let print_expr = quote! {
    #(#before_print)*
    #(#print)*
    #(#after_print)*
  };

  (parse_expr, print_expr)
}
