extern crate proc_macro;

mod fuse_handlers_signatures;

use either::Either;
use proc_macro::TokenStream;
use quote::quote;
use syn::{
    Attribute, ExprBlock, Ident, Result, Token,
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
    token::{Comma, FatArrow, Semi},
};

use fuse_handlers_signatures::*;

struct FnSigInput {
    attrs: Vec<Attribute>,
    name: Ident,
    tail: Either<Semi, ExprBlock>,
}

impl Parse for FnSigInput {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut attrs = Vec::new();
        while input.peek(Token![#]) {
            // Stop if it's an inner attribute #![...]
            if input.peek2(Token![!]) {
                break;
            }
            attrs.extend(input.call(Attribute::parse_outer)?);
        }

        let name: Ident = input.parse()?;
        let _arrow: FatArrow = input.parse()?;

        let tail = if input.peek(Token![;]) {
            Either::Left(input.parse::<Semi>()?)
        } else {
            Either::Right(input.parse::<ExprBlock>()?)
        };

        Ok(FnSigInput { attrs, name, tail })
    }
}

/// Usage:
/// 1. ```fuse_handler_fnsig!{
///        /// doc...
///        init => { _default_implementation_ }
///    }
/// ```
/// 1. ```fuse_handler_fnsig!{
///        /// doc...
///        access => ;
///    }
/// ```
#[proc_macro]
pub fn fuse_handler_fnsig(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as FnSigInput);
    let attrs = input.attrs;
    let tail_tokens: proc_macro2::TokenStream = match input.tail {
        Either::Left(semi) => quote!( #semi ),
        Either::Right(block) => {
            quote!( #block ) // reconstruct braced block
        }
    };

    let fun_impl = get_fuse_handler_fn_impl(
        &input.name.to_string(),
    );
    quote!(
        #(#attrs)*
        #fun_impl #tail_tokens
    )
    .into()
}

struct DelegateFsInput {
    target: Ident,
    _comma: Comma,
    methods: Punctuated<Ident, Comma>,
}

impl Parse for DelegateFsInput {
    fn parse(input: ParseStream) -> Result<Self> {
        let target = input.parse::<Ident>()?;
        let _comma = input.parse::<Comma>()?;

        let content;
        syn::braced!(content in input);

        let methods = Punctuated::<Ident, Comma>::parse_terminated(&content)?;

        Ok(Self {
            target,
            _comma,
            methods,
        })
    }
}

#[proc_macro]
pub fn delegate_fs(input: TokenStream) -> TokenStream {
    let args = syn::parse_macro_input!(input as DelegateFsInput);

    let target = args.target;

    let mut expanded = quote! {};

    for method in &args.methods {
        let fn_impl = get_fuse_handler_fn_impl(&method.to_string());
        let method_expr = make_method_call_expr(&fn_impl);
        expanded.extend(quote! {
            #fn_impl {
                self.#target.#method_expr
            }
        });
    }

    expanded.into()
}