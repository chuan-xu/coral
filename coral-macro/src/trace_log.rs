use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use quote::ToTokens;
use syn::parse::Parse;
use syn::parse_macro_input;
use syn::Expr;
use syn::Ident;
use syn::Lit;
use syn::LitStr;
use syn::Token;

#[derive(Debug)]
enum Kval {
    Expr(Expr),
    Lit(Lit),
}

impl Parse for Kval {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        match input.peek(Lit) {
            true => Ok(Self::Lit(input.parse()?)),
            false => Ok(Self::Expr(input.parse()?)),
        }
    }
}

impl ToTokens for Kval {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let val = match self {
            Kval::Expr(v) => quote! {#v},
            Kval::Lit(v) => quote! {#v},
        };
        tokens.extend(val);
    }
}

#[derive(Debug)]
struct KeyValue {
    key: Ident,
    val: Kval,
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
enum Target {
    None,
    Expr(Expr),
    Lit(Lit),
}

impl ToTokens for Target {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let target = match self {
            Target::None => quote! {},
            Target::Expr(v) => quote! {target: #v,},
            Target::Lit(v) => quote! {target: #v,},
        };
        tokens.extend(target);
    }
}

#[derive(Debug)]
struct Log {
    level: Level,
    target: Target,
    kvs: Vec<KeyValue>,
    msg: LitStr,
    args: Vec<Expr>,
}

impl ToTokens for Log {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let target = &self.target;
        let (kvs, kvs_ext) = if self.kvs.len() == 0 {
            (quote! {}, quote! {trace_id = span.trace_id.0; })
        } else {
            let kvs = &self.kvs;
            (
                quote! {#(#kvs),*;},
                quote! {#(#kvs),*, trace_id = span.trace_id.0; },
            )
        };
        let args = if self.args.len() == 0 {
            quote! {}
        } else {
            let args = &self.args;
            quote! {
                ,#(#args),*
            }
        };
        let level = &self.level;
        let msg = &self.msg;
        tokens.extend(quote! {
            match fastrace::collector::SpanContext::current_local_parent() {
                Some(span) => log::#level!(#target #kvs_ext #msg #args),
                None => log::#level!(#target #kvs #msg #args),
            }
        });
    }
}

impl Parse for Log {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut target = Target::None;
        let mut kvs = Vec::new();
        let mut args = Vec::new();
        if input.peek(Ident) {
            let first_ident: Ident = input.parse()?;
            if first_ident == "target" {
                let _: Token![:] = input.parse()?;
                if input.peek(Lit) {
                    target = Target::Lit(input.parse()?);
                } else {
                    target = Target::Expr(input.parse()?);
                }
                let _: Token![,] = input.parse()?;
            }
            if let Target::None = target {
                let _: Token![=] = input.parse()?;
                kvs.push(KeyValue {
                    key: first_ident,
                    val: input.parse()?,
                });
                if input.peek(Token![,]) {
                    let _: Token![,] = input.parse()?;
                } else if input.peek(Token![;]) {
                    let _: Token![;] = input.parse()?;
                }
            }
        }
        while input.peek(Ident) {
            let key = input.parse()?;
            let _: Token![=] = input.parse()?;
            let val = input.parse()?;
            kvs.push(KeyValue { key, val });
            if input.peek(Token![,]) {
                let _: Token![,] = input.parse()?;
            } else if input.peek(Token![;]) {
                let _: Token![;] = input.parse()?;
            }
        }
        let msg: LitStr = input.parse()?;
        while input.peek(Token![,]) {
            let _: Token![,] = input.parse()?;
            args.push(input.parse()?);
        }
        Ok(Log {
            level: Level::Trace,
            target,
            kvs,
            msg,
            args,
        })
    }
}

#[derive(Debug)]
pub enum Level {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl ToTokens for Level {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let level = match self {
            Level::Error => quote! {error},
            Level::Warn => quote! {warn},
            Level::Info => quote! {info},
            Level::Debug => quote! {debug},
            Level::Trace => quote! {trace},
        };
        tokens.extend(level);
    }
}

pub fn parse_log(input: TokenStream, level: Level) -> TokenStream {
    let mut parsed = parse_macro_input!(input as Log);
    parsed.level = level;
    let mut stream = TokenStream2::new();
    parsed.to_tokens(&mut stream);
    TokenStream::from(stream)
}
