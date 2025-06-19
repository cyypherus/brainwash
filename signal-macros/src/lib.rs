use proc_macro::TokenStream;
use quote::quote;
use std::sync::atomic::{AtomicUsize, Ordering};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn next_id() -> usize {
    COUNTER.fetch_add(1, Ordering::SeqCst)
}

#[proc_macro]
pub fn adsr(_input: TokenStream) -> TokenStream {
    let id = next_id();

    let expanded = quote! {
        crate::adsr(#id)
    };

    TokenStream::from(expanded)
}

#[proc_macro]
pub fn sin(_input: TokenStream) -> TokenStream {
    let id = next_id();

    let expanded = quote! {
        crate::sin(#id)
    };

    TokenStream::from(expanded)
}

#[proc_macro]
pub fn squ(_input: TokenStream) -> TokenStream {
    let id = next_id();

    let expanded = quote! {
        crate::squ(#id)
    };

    TokenStream::from(expanded)
}

#[proc_macro]
pub fn tri(_input: TokenStream) -> TokenStream {
    let id = next_id();

    let expanded = quote! {
        crate::tri(#id)
    };

    TokenStream::from(expanded)
}

#[proc_macro]
pub fn seq(input: TokenStream) -> TokenStream {
    let id = next_id();
    let input: proc_macro2::TokenStream = input.into();

    let expanded = quote! {
        crate::sequence(#id, #input)
    };

    TokenStream::from(expanded)
}

#[proc_macro]
pub fn saw(_input: TokenStream) -> TokenStream {
    let id = next_id();

    let expanded = quote! {
        crate::saw(#id)
    };

    TokenStream::from(expanded)
}

#[proc_macro]
pub fn rsaw(_input: TokenStream) -> TokenStream {
    let id = next_id();

    let expanded = quote! {
        crate::rsaw(#id)
    };

    TokenStream::from(expanded)
}

#[proc_macro]
pub fn clock(_input: TokenStream) -> TokenStream {
    let id = next_id();

    let expanded = quote! {
        crate::clock(#id)
    };

    TokenStream::from(expanded)
}
