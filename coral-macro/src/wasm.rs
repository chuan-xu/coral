use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Data, DeriveInput, Fields};

pub fn wasm_attribute(input: TokenStream) -> TokenStream {
    let derive_input = parse_macro_input!(input as DeriveInput);
    let ident = &derive_input.ident;
    let mut output = TokenStream2::new();
    if let Data::Struct(stu) = derive_input.data {
        if let Fields::Named(names) = stu.fields {
            let mut attrs = TokenStream2::new();
            for name in names.named.iter() {
                if let Some(name_idt) = name.ident.as_ref() {
                    // let mem_ident = &name.ident;
                    let type_idt = &name.ty;
                    let set_fn = format_ident!("set_{}", name_idt);
                    attrs.extend(quote! {
                        #[wasm_bindgen(getter)]
                        pub fn #name_idt(&self) -> #type_idt {
                            self.#name_idt.clone()
                        }
                        #[wasm_bindgen(setter)]
                        pub fn #set_fn(&mut self, #name_idt: #type_idt) {
                            self.#name_idt = #name_idt;
                        }
                    });
                }
            }
            if !attrs.is_empty() {
                output.extend(quote! {
                    #[wasm_bindgen::prelude::wasm_bindgen]
                    impl #ident {
                        #attrs
                    }
                });
            }
        }
    }
    TokenStream::from(output)
}
