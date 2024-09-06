use super::*;

pub(crate) fn derive_opio_(
  tokens: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
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
      let info = FieldsInfo::new(None, &fields, "opio");
      let keys = ["input", "output", "attr", "wrap"];
      for item in &info.infos {
        let traits = item.args_map.contains_key("input") as u8
          + item.args_map.contains_key("output") as u8
          + item.args_map.contains_key("attr") as u8
          + item.args_map.contains_key("wrap") as u8
          + item.args_map.contains_key("wrap_input") as u8
          + item.args_map.contains_key("wrap_output") as u8;
        if traits > 1 {
          proc_panic!(
            item.field.span().unwrap(),
            "Only one of input, output, attr, wrap, wrap_input and wrap_output is allowed"
          );
        }
      }
      let find_all =
        |name: &str, use_mut: bool, wrap_len: &str, wrap_get: &str| {
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
            } else if info.args_map.contains_key("wrap")
              || (info.args_map.contains_key("wrap_input") && name == "input")
              || (info.args_map.contains_key("wrap_output") && name == "output")
            {
              if use_mut {
                res.push(quote! { <&mut self.#iname, #wrap_len, #wrap_get>})
              } else {
                res.push(quote! { <&self.#iname, #wrap_len, #wrap_get> });
              }
            }
          }
          quote! { #(#res),* }
        };

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
