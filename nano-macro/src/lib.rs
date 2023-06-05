extern crate proc_macro;
use proc_macro_error::{proc_macro_error, abort};

use proc_macro2::{TokenStream, Delimiter, Group, TokenTree, Punct, Spacing, Ident};
use quote::quote;
use std::borrow::Cow;

// #[proc_macro]
// fn noop(_input: proc_macro::TokenStream) -> proc_macro::TokenStream {
//     proc_macro::TokenStream::new()
// }

#[proc_macro_error]
#[proc_macro]
pub fn key(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let (result, leftover) = partial_key(input.into());
    if ! leftover.is_empty() {
        abort!(leftover, "Left over tokens");
    }
    result.into()
}

#[proc_macro_error]
#[proc_macro]
pub fn keyseq(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let mut input: TokenStream = input.into();
    let mut keys = vec![];
    loop {
        let (result, leftover) = partial_key(input);
        keys.push(result);
        if leftover.is_empty() {
            break;
        }
        input = leftover;
    }
    quote! {
        vec![#(#keys),*]
    }.into()
}

fn partial_key(input: TokenStream) -> (TokenStream, TokenStream) {
    // let input: TokenStream = input.into();
    let mut r = TokenStream::new();
    let mut i = input.into_iter().peekable();
    let mut key_code: Option<TokenStream> = None;

    fn is_dash(tree: &TokenTree) -> bool {
        match tree {
            TokenTree::Punct(ref punct) => punct.as_char() == '-',
            _ => false
        }
    }

    while let Some(tree) = i.next() {
        if i.peek().is_none() || (!is_dash(&tree) && !is_dash(i.peek().unwrap())) {
            key_code = match tree {
                TokenTree::Literal(ref literal) => {
                    let x = literal.to_string();
                    if x.len() == 1 && x.parse::<u8>().is_ok() {
                        let key = Ident::new(&format!("Key{x}"), literal.span());
                        Some(quote! { ::bevy::prelude::KeyCode::#key })
                    } else {
                        match x.as_str() {
                            "'\\''" => Some(quote! { ::bevy::prelude::KeyCode::Apostrophe }),
                            "'`'" => Some(quote! { ::bevy::prelude::KeyCode::Grave }),
                            "'\\\\'" => Some(quote! { ::bevy::prelude::KeyCode::Backslash }),
                            _ => todo!("literal char {x} {:?}", literal),
                        }
                    }
                    // else {
                    //     todo!("literal {:?}", literal);
                    // }
                },
                TokenTree::Punct(ref punct) => {
                    let name : Option<Cow<'static, str>> = match punct.as_char() {
                        ';' => Some("Semicolon".into()),
                        ':' => Some("Colon".into()),
                        ',' => Some("Comma".into()),
                        '.' => Some("Period".into()),
                        '^' => Some("Caret".into()),
                        '=' => Some("Equals".into()),
                        '/' => Some("Slash".into()),
                        '-' => Some("Minus".into()),
                        '*' => Some("Asterisk".into()),
                        '+' => Some("Plus".into()),
                        '@' => Some("At".into()),
                        // _ => None

                        _ => todo!("punct {:?}", punct),
                    };
                    name.as_ref().map(|n| {
                        let token = Ident::new(n, punct.span());
                        quote! {::bevy::prelude::KeyCode::#token }
                    })
                },
                TokenTree::Ident(ref ident) => {
                    // Some(quote! { ::bevy::prelude::KeyCode::#ident })
                    let label = ident.to_string();
                    if label.len() == 1 {
                        let name : Option<Cow<'static, str>>
                            = match label.chars().next().unwrap() {
                            x @ 'A'..='Z' | x @ 'a'..='z' => {
                                let s = x.to_ascii_uppercase().to_string();
                                // let upper = Ident::new(&s, ident.span());
                                Some(s.into())
                                // Some(quote! {::bevy::prelude::KeyCode::#upper })
                            },
                                '_' => Some("Underline".into()),
                            // Identifiers can't start with a number.
                            // x @ '0'..='9' => {
                            //     let key = Ident::new(&format!("Key{x}"), ident.span());
                            //     Some(quote! {::bevy::prelude::KeyCode::#key })
                            // },
                            _ => todo!("ident {:?}", ident),
                        };
                        name.as_ref().map(|n| {
                            let token = Ident::new(n, ident.span());
                            quote! {::bevy::prelude::KeyCode::#token }
                        })
                    } else {
                        Some(quote! { ::bevy::prelude::KeyCode::#ident})
                    }
                },
                _ => None
            };
            break;
        } else {
            let replacement = match tree {
                TokenTree::Ident(ref ident) => {
                    match ident.to_string().as_str() {
                        "ctrl" => Some(TokenTree::Group(Group::new(Delimiter::None,
                                                            quote! { ::bevy_nano_console::hotkey::Modifiers::Control }))),
                        "alt" => Some(TokenTree::Group(Group::new(Delimiter::None,
                                                            quote! { ::bevy_nano_console::hotkey::Modifiers::Alt }))),
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
    // This will add an empty to finish the expression:
    //
    //    ctrl-alt-EMPTY -> Control | Alt | EMPTY.
    //
    //  And it will provide a valid Modifier when none have been provided.
    r.extend([quote! { ::bevy_nano_console::hotkey::Modifiers::empty() }]);
    let key_code = key_code.expect("No ::bevy::prelude::KeyCode found.");
    (quote! {
        ::bevy_nano_console::hotkey::Key(#r, #key_code)
    },
     TokenStream::from_iter(i))
    // r.into()
}
