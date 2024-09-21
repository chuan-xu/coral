use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};
use syn::{parse_macro_input, Data, DeriveInput, Type, TypePath};

pub fn assign_basic(input: TokenStream) -> TokenStream {
    if let Type::Path(TypePath { path, .. }) = parse_macro_input!(input as Type) {
        if let Some(segment) = path.segments.get(0) {
            let ident = &segment.ident;
            return TokenStream::from(quote! {
                impl EnvAssignToml for #ident {
                    fn assign(&mut self, prefix: Option<&str>) -> std::result::Result<(), toml::de::Error> {
                        if let Some(prefix) = prefix {
                            if let Ok(v) = std::env::var(prefix) {
                                *self = toml::from_str(&v)?;
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
    let t = parse_macro_input!(input as DeriveInput);
    if let Data::Struct(s) = t.data {
        for field in s.fields {
            eprintln!("[++] {:?}", field.ident);
        }
    }
    TokenStream::from(quote! {
        // const _: () = {};
        impl Tls {}
    })
}
