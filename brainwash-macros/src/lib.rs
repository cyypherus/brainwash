use proc_macro::TokenStream;
use quote::quote;
use std::sync::atomic::{AtomicUsize, Ordering};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn next_id() -> usize {
    COUNTER.fetch_add(1, Ordering::SeqCst)
}

#[proc_macro]
pub fn id(_input: TokenStream) -> TokenStream {
    let id = next_id();
    let expanded = quote! {
        #id
    };
    TokenStream::from(expanded)
}
