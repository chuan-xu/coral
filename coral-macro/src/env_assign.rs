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
    if let Data::Struct(s) = derive_input.data {
        for field in s.fields.iter() {
            if let Some(field_ident) = field.ident.as_ref() {
                let this = field_ident.to_string().to_uppercase();
                fields.extend(quote! {
                    self.#field_ident.assign(Some(format!("{}{}", &prefix, #this).as_str()))?;
                });
            }
        }
    }
    let ident = &derive_input.ident;
    TokenStream::from(quote! {
        const _: () = {
            impl EnvAssignToml for  #ident {
                fn assign(&mut self, prefix: Option<&str>) -> std::result::Result<(), serde_json::Error> {
                    let prefix = match prefix {
                        Some(v) => format!("{}_", v),
                        None => "".into(),
                    };
                    #fields
                    Ok(())
                }
            }
        };
        // impl Tls {}
    })
}
