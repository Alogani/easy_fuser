extern crate proc_macro;

mod fuse_handlers_signatures;

use proc_macro::TokenStream;
use quote::quote;
use syn::{
    Attribute, ExprBlock, Ident, LitStr, Result, Token, parse::{Parse, ParseStream}, parse_macro_input,
    token::{FatArrow, Group, Semi}
};
use either::Either;

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

        Ok(FnSigInput {
            attrs,
            name,
            tail,
        })
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
            quote!( #block )   // reconstruct braced block
        }
    };

    let fun_impl = fuse_handlers_signatures::get_fuse_handler_fn_impl(
        &input.name.to_string(),
            Some(tail_tokens),
    );
    quote!(
        #(#attrs)*
        #fun_impl
    ).into()
}

/*
#[proc_macro]
pub fn delegate(input: TokenStream) -> TokenStream {
    let parsed: syn::ExprTuple = parse_macro_input!(input);
    let field_expr: Expr = parsed.elems[0].clone(); // self.fuse_handler
    let methods_vec: syn::ExprVec = syn::parse2(parsed.elems[1].to_token_stream()).unwrap();

    let mut delegations = vec![];
    for method in methods_vec.elems {
        let method_ident = if let syn::Expr::Path(p) = method { p.path.get_ident().unwrap().clone() } else { panic!("Expected ident") };
        let method_str = method_ident.to_string();

        // Find matching signature
        let sig = METHODS.iter().find(|(name, _)| *name == method_str).map(|(_, sig)| sig).unwrap_or_else(|| panic!("Unknown method: {}", method_str));
        let sig_parsed: syn::Signature = syn::parse_str(sig).unwrap();

        // Generate delegation
        let args: Vec<Ident> = sig_parsed.inputs.iter().skip(1).map(|arg| if let syn::FnArg::Typed(t) = arg { t.pat.as_ref().clone() } else { panic!() }).collect(); // Skip &self
        let arg_idents: Vec<_> = args.iter().map(|pat| if let syn::Pat::Ident(i) = pat { i.ident.clone() } else { panic!() }).collect();

        delegations.push(quote! {
            #sig_parsed {
                #field_expr.#method_ident(#(#arg_idents),*)
            }
        });
    }

    quote!(#(#delegations)*).into()
}
    */
