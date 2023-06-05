extern crate proc_macro;

use proc_macro::{TokenStream, Delimiter, Group, TokenTree, Punct, Spacing};
use quote::quote;
// use syn::{self, parse_macro_input};

#[proc_macro]
pub fn key(input: TokenStream) -> TokenStream {

    let ts: TokenStream = quote! {
        Key::new(KeyCode::A, Modifiers::empty())
    }.into();
    eprintln!("{:?}", ts);
    let mut r = TokenStream::new();
    let mut i = input.into_iter().peekable();
    let mut key_code: Option<TokenStream> = None;
    while let Some(tree) = i.next() {
        if i.peek().is_none() {
            key_code = match tree {
                TokenTree::Ident(ref ident) => {
                    match ident.to_string().as_str() {
                        "A" => Some(quote! { KeyCode::A }.into()),
                        _ => None
                    }
                },
                _ => None
            };
        } else {
            let replacement = match tree {
                TokenTree::Ident(ref ident) => {
                    match ident.to_string().as_str() {
                        "ctrl" => Some(TokenTree::Group(Group::new(Delimiter::None,
                                                            quote! { Modifiers::Control }.into()))),
                        "alt" => Some(TokenTree::Group(Group::new(Delimiter::None,
                                                            quote! { Modifiers::Alt }.into()))),
                        _ => None
                    }
                },
                TokenTree::Punct(ref punct) => {
                    match punct.as_char() {
                        '-' => Some(TokenTree::Punct(Punct::new('|', Spacing::Alone))),
                        _ => None
                    }
                }
                _ => None
            };
            r.extend([replacement.unwrap_or(tree)]);
        }
    }
    let key_code = key_code.unwrap();
    // quote! {
    //     Key::new(#key_code, #r)
    // }.into()
    r
}
