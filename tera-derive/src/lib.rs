// The `quote!` macro requires deep recursion.
#![recursion_limit = "196"]

extern crate proc_macro;

use bae::FromAttributes;
use fnv::FnvHasher;
use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::punctuated::Punctuated;
use syn::token::Comma;
use syn::{Fields, ItemStruct, LitInt, LitStr, Path};

use std::cmp::Ordering;
use std::hash::{Hash, Hasher};

type UnitFields = Punctuated<syn::Field, Comma>;

struct Field {
    hash: u64,
    field: TokenStream2,
    callback: Option<Path>,
}

impl PartialEq for Field {
    fn eq(&self, other: &Field) -> bool {
        self.hash == other.hash
    }
}

impl Eq for Field {}

impl PartialOrd for Field {
    fn partial_cmp(&self, other: &Field) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Field {
    fn cmp(&self, other: &Field) -> Ordering {
        self.hash.cmp(&other.hash)
    }
}

#[derive(FromAttributes)]
struct ContentAttributes {
    skip: Option<()>,
    flatten: Option<()>,
    rename: Option<LitStr>,
    callback: Option<Path>,
}

#[proc_macro_derive(Content, attributes(tera))]
pub fn content_derive(input: TokenStream) -> TokenStream {
    let item: ItemStruct =
        syn::parse(input).expect("#[derive(Content)] can be only applied to structs");

    // panic!("{:#?}", item);

    let name = &item.ident;
    let generics = &item.generics;
    let type_params = item.generics.type_params();
    let unit_fields = UnitFields::new();

    let mut errors = Vec::new();

    let fields = match item.fields {
        Fields::Named(fields) => fields.named.into_iter(),
        Fields::Unnamed(fields) => fields.unnamed.into_iter(),
        _ => unit_fields.into_iter(),
    };

    let mut flatten = Vec::new();
    let mut fields = fields
        .enumerate()
        .filter_map(|(index, field)| {
            let mut callback = None;
            let mut rename = None;
            let mut skip = false;

            match ContentAttributes::try_from_attributes(&field.attrs) {
                Ok(Some(content_attributes)) => {
                    if content_attributes.skip.is_some() {
                        skip = true;
                    }
                    if content_attributes.flatten.is_some() {
                        flatten.push(field.ident.as_ref().map_or_else(
                            || {
                                let index = index.to_string();
                                let lit = LitInt::new(&index, Span::call_site());
                                quote!(#lit)
                            },
                            |ident| quote!(#ident),
                        ));
                        skip = true;
                    }
                    if let Some(lit_str) = content_attributes.rename {
                        rename = Some(lit_str.value());
                    }
                    if let Some(path) = content_attributes.callback {
                        callback = Some(path);
                    }
                }
                Ok(None) => (),
                Err(err) => errors.push(err),
            };

            if skip {
                return None;
            }

            let (name, field) = field.ident.as_ref().map_or_else(
                || {
                    let index = index.to_string();
                    let lit = LitInt::new(&index, Span::call_site());
                    let name = rename.as_ref().cloned().unwrap_or(index);
                    (name, quote!(#lit))
                },
                |ident| {
                    let name = rename.as_ref().cloned().unwrap_or_else(|| ident.to_string());
                    (name, quote!(#ident))
                },
            );

            let mut hasher = FnvHasher::default();
            name.hash(&mut hasher);
            let hash = hasher.finish();

            Some(Field { hash, field, callback })
        })
        .collect::<Vec<_>>();

    if !errors.is_empty() {
        let errors: Vec<_> = errors.into_iter().map(|e| e.to_compile_error()).collect();
        return quote! {
            fn _tera_derive_compile_errors() {
                #(#errors)*
            }
        }
        .into();
    }

    fields.sort_unstable();

    let render = fields.iter().map(|Field { field, hash, callback, .. }| {
        if let Some(callback) = callback {
            quote! {
                #hash => #callback(&self.#field, encoder).map(|_| true),
            }
        } else {
            quote! {
                #hash => self.#field.render(encoder).map(|_| true),
            }
        }
    });

    let flatten = &*flatten;
    let fields = fields.iter().map(|Field { field, .. }| field);

    let where_clause =
        type_params.map(|param| quote!(#param: Content)).collect::<Vec<_>>();
    let where_clause =
        if !where_clause.is_empty() { quote!(where #(#where_clause),*) } else { quote!() };

    // FIXME: decouple lifetimes from actual generics with trait boundaries
    let tokens = quote! {
        impl #generics Content for #name #generics #where_clause {
            #[inline]
            fn capacity_hint(&self) -> usize {
                0 #( + self.#fields.capacity_hint(tpl) )*
            }


            #[inline]
            fn render(&self, write: &mut dyn Write) -> std::io::Result<()>
            {
                match hash {
                    #( #render )*
                    _ => Ok(
                        #( self.#flatten.render(write)? ||)*
                        false
                    )
                }
            }
        }
    };
    TokenStream::from(tokens)
}
