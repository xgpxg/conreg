//! Conreg Feign Macro
//!
//! This crate provides procedural macros for creating Feign-like declarative HTTP clients.

use proc_macro::TokenStream;

mod feign_client;

/// Feign client attribute macro
#[proc_macro_attribute]
pub fn feign_client(args: TokenStream, input: TokenStream) -> TokenStream {
    feign_client::feign_client_impl(args, input)
}

/// GET request annotation
#[proc_macro_attribute]
pub fn get(_args: TokenStream, input: TokenStream) -> TokenStream {
    input
}

/// POST request annotation
#[proc_macro_attribute]
pub fn post(_args: TokenStream, input: TokenStream) -> TokenStream {
    input
}

/// PUT request annotation
#[proc_macro_attribute]
pub fn put(_args: TokenStream, input: TokenStream) -> TokenStream {
    input
}

/// DELETE request annotation
#[proc_macro_attribute]
pub fn delete(_args: TokenStream, input: TokenStream) -> TokenStream {
    input
}

/// PATCH request annotation
#[proc_macro_attribute]
pub fn patch(_args: TokenStream, input: TokenStream) -> TokenStream {
    input
}
