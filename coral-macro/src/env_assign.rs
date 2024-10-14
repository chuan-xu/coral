use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Type, TypePath};

pub fn assign_basic(input: TokenStream) -> TokenStream {
    if let Type::Path(TypePath { path, .. }) = parse_macro_input!(input as Type) {
        if let Some(segment) = path.segments.get(0) {
            let ident = &segment.ident;
            return TokenStream::from(quote! {
                impl EnvAssignToml for #ident {
                    fn assign(&mut self, prefix: Option<&str>) -> std::result::Result<(), serde_json::Error> {
                        if let Some(prefix) = prefix {
                            if let Ok(v) = std::env::var(prefix) {
                                *self = serde_json::from_str(&v)?;
                            }
                        }
                        Ok(())
                    }
                }
            });
        }
    }
    TokenStream::from(quote! {})
}

pub fn assign_struct(input: TokenStream) -> TokenStream {
    let derive_input = parse_macro_input!(input as DeriveInput);
    let mut fields = TokenStream2::new();
    match derive_input.data {
        Data::Struct(s) => {
            fields.extend(quote! {
                let prefix = match prefix {
                    Some(v) => format!("{}_", v),
                    None => "".into(),
                };
            });
            for field in s.fields.iter() {
                if let Some(field_ident) = field.ident.as_ref() {
                    let this = field_ident.to_string().to_uppercase();
                    fields.extend(quote! {
                        self.#field_ident.assign(Some(format!("{}{}", &prefix, #this).as_str()))?;
                    });
                }
            }
            fields.extend(quote! {
                Ok(())
            });
        }
        Data::Enum(e) => {
            let mut unit_var = Vec::new();
            let mut item_var = Vec::new();
            for var in e.variants.iter() {
                let vident = &var.ident;
                if let syn::Fields::Unit = var.fields {
                    // unit_var.push(var.ident);
                    let vident_str = &vident.to_string();
                    unit_var.push(quote! {
                        if v == #vident_str {
                            *self = Self::#vident;
                            return Ok(());
                        }
                    });
                } else {
                    let this = vident.to_string().to_uppercase();
                    item_var.push(quote! {
                        Self::#vident(v) => v.assign(Some(format!("{}{}", &prefix2, #this).as_str()))
                    });
                }
            }
            if unit_var.len() != 0 {
                fields.extend(quote! {
                    let prefix1 = prefix.unwrap();
                    if let Ok(v) = std::env::var(prefix1) {
                        #(#unit_var)*
                    }
                });
            }
            if item_var.len() != 0 {
                item_var.push(quote! {
                    _ => { Ok(()) }
                });
                fields.extend(quote! {
                    let prefix2 = match prefix {
                        Some(v) => format!("{}_", v),
                        None => "".into()
                    };
                    match self {
                        #(#item_var),*
                    }
                });
            } else {
                fields.extend(quote! {
                    Ok(())
                });
            }
        }
        Data::Union(_) => {
            // TODO:
            todo!()
        }
    };
    let ident = &derive_input.ident;
    TokenStream::from(quote! {
        const _: () = {
            impl EnvAssignToml for  #ident {
                fn assign(&mut self, prefix: Option<&str>) -> std::result::Result<(), serde_json::Error> {
                    #fields
                }
            }
        };
        // impl Tls {}
    })
}
