use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use quote::ToTokens;
use syn::parse::Parse;
use syn::parse_macro_input;
use syn::punctuated::Punctuated;
use syn::Expr;
use syn::ExprBinary;
use syn::ExprMacro;
use syn::Ident;
use syn::LitStr;
use syn::Token;

// pub mod trace_log;

#[derive(Debug)]
struct KeyValue {
    key: Ident,
    val: Expr,
}

impl Parse for KeyValue {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let key = input.parse()?;
        let _eq: Token![=] = input.parse()?;
        let val = input.parse()?;
        Ok(Self { key, val })
    }
}

impl ToTokens for KeyValue {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let key = &self.key;
        let val = &self.val;
        tokens.extend(quote! {#key = #val});
    }
}

#[derive(Debug)]
struct Log {
    target: Option<Expr>,
    kvs: Vec<KeyValue>,
    msg: LitStr,
    args: Vec<Expr>,
}

impl ToTokens for Log {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let target = match self.target.as_ref() {
            Some(t) => quote! {target: #t,},
            None => quote! {},
        };
        let kvs = if self.kvs.len() == 0 {
            quote! {}
        } else {
            let kvs = &self.kvs;
            quote! {
                #(#kvs),*;
            }
        };
        let args = if self.args.len() == 0 {
            quote! {}
        } else {
            let args = &self.args;
            quote! {
                ,#(#args),*
            }
        };
        let msg = &self.msg;
        tokens.extend(quote! {
            #target #kvs #msg #args;
        });
    }
}

impl Parse for Log {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut target = None;
        let mut kvs = Vec::new();
        let mut args = Vec::new();
        if input.peek(Ident) {
            let first_ident: Ident = input.parse()?;
            if first_ident == "target" {
                let _: Token![:] = input.parse()?;
                target = Some(input.parse()?);
                let _: Token![,] = input.parse()?;
            }
            if target.is_none() {
                let _: Token![=] = input.parse()?;
                kvs.push(KeyValue {
                    key: first_ident,
                    val: input.parse()?,
                });
            }
        }
        if input.peek(Token![,]) || input.peek(Ident) {
            if input.peek(Token![,]) {
                let _: Token![,] = input.parse()?;
            }
            while input.peek(Ident) {
                let key = input.parse()?;
                let _: Token![=] = input.parse()?;
                let val = input.parse()?;
                kvs.push(KeyValue { key, val });
            }
            let _: Token![;] = input.parse()?;
        } else if input.peek(Token![;]) {
            let _: Token![;] = input.parse()?;
        }
        let msg: LitStr = input.parse()?;
        while input.peek(Token![,]) {
            let _: Token![,] = input.parse()?;
            args.push(input.parse()?);
        }
        Ok(Log {
            target,
            kvs,
            msg,
            args,
        })
    }
}

#[proc_macro]
pub fn trace_info(input: TokenStream) -> TokenStream {
    let c = input.clone();
    let parsed = parse_macro_input!(input as Log);
    let mut stream = TokenStream2::new();
    parsed.to_tokens(&mut stream);
    let t = stream.to_string();
    eprintln!("==============={:?}", t);
    c
    // TokenStream::from(stream)
}

// #[proc_macro]
// pub fn trace_info(input: TokenStream) -> TokenStream {
//     // eprintln!("{:?}", input);
//     let mut parsed = parse_macro_input!(input as Log);
//     eprintln!("######################");
//     let mut ts = Vec::new();
//     while let Some(v) = parsed.pairs.pop() {
//         let v = v.into_value();
//         let key = &v.key;
//         let eq = &v.eq;
//         let val = &v.val;
//         let s = quote! { #key #eq #val};
//         ts.push(s);
//     }
//     ts.reverse();
//     let t = quote!(
//         #(#ts),*
//     )
//     .to_string();
//     TokenStream::from(quote! {
//         #t
//     })
// }

// #[proc_macro]
// pub fn trace_info(input: TokenStream) -> TokenStream {
//     eprintln!("{:?}", input);
//     let parsed = parse_macro_input!(input as Log);
//     eprintln!("====={:?}", parsed);
//     let mut ts = TokenStream2::new();
//     parsed.to_tokens(&mut ts);
//     TokenStream::from(ts)
// }
