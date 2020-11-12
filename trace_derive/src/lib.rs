extern crate proc_macro;
extern crate quote;

use proc_macro::TokenStream;
use syn::{parse_macro_input, Data, DeriveInput, Field, Ident, Meta};

use quote::quote;

#[proc_macro_derive(GcLayout, attributes(trace_end, no_trace))]
pub fn gc_layout_derive(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);

    let mut fields = Vec::new();
    let mut found_no_trace_attr = false;
    let mut found_trace_end_attr = false;

    match ast.data {
        Data::Struct(ref data) => {
            for (idx, field) in data.fields.iter().enumerate() {
                let field_access = match field.ident {
                    Some(ref ident) => quote! { &self.#ident },
                    None => quote! { &self.#(syn::Index::from(#idx)) },
                };

                let bit_fiddling = if !should_trace(field) {
                    found_no_trace_attr = true;
                    quote! {
                      let size = ::std::mem::size_of_val(#field_access) / ::std::mem::size_of::<usize>();
                      for w in 0..size {
                          bitmap &= !(1 << cur);
                          cur += 1;
                      }
                    }
                } else {
                    quote! {
                      cur += ::std::mem::size_of_val(#field_access) / ::std::mem::size_of::<usize>();
                    }
                };

                fields.push(bit_fiddling);

                if is_trace_len(field) {
                    found_trace_end_attr = true;
                    break;
                }
            }
        }
        Data::Enum(_) => unimplemented!("Enums not supported yet"),
        Data::Union(_) => unimplemented!("Untagged unions not supported yet"),
    };

    let return_value = if found_trace_end_attr {
        quote! { crate::gc::LayoutInfo::PartiallyTraceable(cur) }
    } else if found_no_trace_attr {
        quote! { crate::gc::LayoutInfo::Precise { bitmap, trace_len: cur } }
    } else {
        quote! { crate::gc::LayoutInfo::Conservative }
    };

    let name = &ast.ident;
    let generics = &ast.generics;
    let (impl_generics, ty_params, where_clause) = generics.split_for_impl();

    let expanded = quote! {
        unsafe impl #impl_generics crate::gc::GcLayout for #name #ty_params #where_clause {
            fn layout_info(&self) -> crate::gc::LayoutInfo {
                let mut bitmap = 0xFFFFFFFF_usize;
                let mut cur = 0;
                #(#fields)*
                #return_value
            }
        }
    };

    TokenStream::from(expanded)
}

fn is_trace_len(field: &Field) -> bool {
    for attr in field.attrs.iter() {
        let option = attr.parse_meta().unwrap();
        match option {
            Meta::Path(ref p) if p.is_ident("trace_end") => return true,
            _ => continue,
        }
    }
    false
}

fn should_trace(field: &Field) -> bool {
    for attr in field.attrs.iter() {
        let option = attr.parse_meta().unwrap();
        match option {
            Meta::Path(ref p) if p.is_ident("no_trace") => return false,
            _ => continue,
        }
    }
    true
}
