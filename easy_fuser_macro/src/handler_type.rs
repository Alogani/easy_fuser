extern crate proc_macro;

use syn::LitStr;

#[derive(Clone, Copy)]
pub(crate) enum HandlerType {
    Async,
    Serial,
    Parallel,
}

pub(crate) fn parse_handler_type(input: LitStr) -> HandlerType {
    match input.value().as_str() {
        "async" => HandlerType::Async,
        "serial" => HandlerType::Serial,
        "parallel" => HandlerType::Parallel,
        _ => panic!("Invalid handler type. Use 'async', 'serial', or 'parallel'"),
    }
}
