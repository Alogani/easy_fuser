extern crate proc_macro;

use proc_macro::TokenStream;
use syn::{parse_macro_input, LitStr};

mod handler_type;
use handler_type::parse_handler_type;

mod fuse_driver;
use fuse_driver::generate_fuse_driver_implementation;
mod fuse_handler;
use fuse_handler::generate_fuse_handler_trait;

#[proc_macro]
pub fn implement_fuse_handler(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as LitStr);
    let handler_type = parse_handler_type(input);

    let trait_impl = generate_fuse_handler_trait(handler_type);

    trait_impl.into()
}

#[proc_macro]
pub fn implement_fuse_driver(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as LitStr);
    let handler_type = parse_handler_type(input);

    let struct_impl = generate_fuse_driver_implementation(handler_type);

    struct_impl.into()
}
