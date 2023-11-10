// extern crate bevy;
// #[cfg(test)]
use bevy::prelude::*;
use bevy_nano_console::hotkey::*;
use nano_macro::*;
// #[cfg(test)]
#[allow(unused_must_use)]
#[test]
fn test_key_eq() {
    let a: Key = KeyCode::A.into();
    let b: Key = KeyCode::A.into();
    assert_eq!(a, b);
    assert!(a == b);
}

// #[should_panic]
#[test]
fn test_bad_key_macro() {
    // assert_eq!(Key(KeyCode::A, Modifiers::Control),
}

#[allow(unused_must_use)]
#[test]
fn test_keyseq_macro() {
    assert_eq!(vec![Key(Modifiers::empty(), KeyCode::A)], keyseq! { A });
    assert_eq!(
        vec![
            Key(Modifiers::empty(), KeyCode::A),
            Key(Modifiers::empty(), KeyCode::B),
        ],
        keyseq! { A B }
    );
}

#[allow(unused_must_use)]
#[test]
fn test_key_macro() {
    assert_eq!(Key(Modifiers::Control, KeyCode::B), key! { ctrl-B });
    assert_eq!(Key(Modifiers::Control, KeyCode::Key1), key! { ctrl-1 });
    assert_eq!(Key(Modifiers::Control, KeyCode::Key2), key! { ctrl-2 });
    assert_eq!(Key(Modifiers::Control, KeyCode::F2), key! { ctrl-F2 });
    // assert_eq!(Key(Modifiers::Control, KeyCode::F2), key!{ ctrl-f2 });
    assert_eq!(Key(Modifiers::Control, KeyCode::Semicolon), key! { ctrl-; });
    assert_eq!(Key(Modifiers::Control, KeyCode::Caret), key! { ctrl-^ });
    // assert_eq!(Key(Modifiers::Control, KeyCode::Colon), key! { ctrl-: });
    assert_eq!(Key(Modifiers::Control | Modifiers::Shift, KeyCode::Semicolon), key! { ctrl-: });
    assert_eq!(Key(Modifiers::Control, KeyCode::Equals), key! { ctrl-= });
    assert_eq!(Key(Modifiers::Control, KeyCode::Comma), key! { ctrl-, });
    assert_eq!(Key(Modifiers::Control, KeyCode::Period), key! { ctrl-. });
    assert_eq!(Key(Modifiers::Control, KeyCode::Slash), key! { ctrl-/ });
    assert_eq!(Key(Modifiers::Control, KeyCode::Minus), key! { ctrl-- });
    assert_eq!(Key(Modifiers::Control, KeyCode::Underline), key! { ctrl-_ });

    assert_eq!(
        Key(Modifiers::Control | Modifiers::Shift, KeyCode::A),
        key! { ctrl-shift-A }
    );
    // assert_eq!(Key(Modifiers::Control, KeyCode::A), key!{ ctrl-A });
    assert_eq!(Key(Modifiers::Super, KeyCode::A), key! { super-A });
    assert_eq!(Key(Modifiers::Control, KeyCode::A), key! { ctrl-A }); // Allow lowercase or demand lowercase?
    assert_eq!(Key(Modifiers::empty(), KeyCode::A), key! { A });
    let k: Key = KeyCode::A.into();
    assert_eq!(k, key! { A });
    assert_eq!(
        Key(Modifiers::Control, KeyCode::Asterisk),
        key! { ctrl-Asterisk }
    ); // All bevy KeyCode names work.
    assert_eq!(Key(Modifiers::Control, KeyCode::Asterisk), key! { ctrl-* }); // with some short hand.

    assert_eq!(Key(Modifiers::Control, KeyCode::Plus), key! { ctrl-+ });
    assert_eq!(Key(Modifiers::Control, KeyCode::At), key! { ctrl-@ });
    assert_eq!(Key(Modifiers::Control, KeyCode::Grave), key! { ctrl-'`' });
    assert_eq!(
        Key(Modifiers::Control, KeyCode::Backslash),
        key! { ctrl-'\\' }
    );
    assert_eq!(
        Key(Modifiers::Control, KeyCode::Escape),
        key! { ctrl-Escape }
    );
    // assert_eq!(Key(Modifiers::Control, KeyCode::Escape), key!{ ctrl-Esc });
    assert_eq!(
        Key(Modifiers::Control | Modifiers::Alt, KeyCode::A),
        key! { ctrl-alt-A }
    );
    assert_eq!(Key(Modifiers::empty(), KeyCode::A), key! { A });
    assert_eq!(
        Key(Modifiers::Control | Modifiers::Alt, KeyCode::A),
        key! { ctrl-alt-A }
    );
    assert_eq!(
        Key(Modifiers::Control | Modifiers::Alt, KeyCode::A),
        key! { ctrl-alt-A }
    );
    assert_eq!(
        Key(Modifiers::Control | Modifiers::Alt, KeyCode::Semicolon),
        key! { ctrl-alt-Semicolon }
    );
    // assert_eq!(Key(Modifiers::Control | Modifiers::Alt, KeyCode::Semicolon), key!{ ctrl-alt-semicolon });
    assert_eq!(
        Key(Modifiers::Control | Modifiers::Alt, KeyCode::Semicolon),
        key! { ctrl-alt-; }
    );
    assert_eq!(
        Key(Modifiers::Control | Modifiers::Alt | Modifiers::Shift, KeyCode::Semicolon),
        key! { ctrl-alt-: }
    );
    assert_eq!(
        Key(Modifiers::Control | Modifiers::Alt, KeyCode::Slash),
        key! { ctrl-alt-/ }
    );

    // assert_eq!(Modifiers::Control | Modifiers::Alt, key!{ ctrl-alt });
    // assert_eq!(Modifiers::Control | Modifiers::Alt, key!{ ctrl - alt });
    // assert_eq!(Modifiers::Control | Modifiers::Alt, key!{ ctrl - alt });
}

#[allow(unused_must_use)]
#[test]
fn test_keyseq() {
    assert_eq!(
        vec![Key(Modifiers::Control, KeyCode::A)],
        keyseq! { ctrl-A }
    );
    assert_eq!(
        vec![Key(Modifiers::Control, KeyCode::A)],
        keyseq! { ctrl-ctrl-A }
    );
    assert_eq!(
        vec![
            Key(Modifiers::Control, KeyCode::A),
            Key(Modifiers::Alt, KeyCode::B)
        ],
        keyseq! { ctrl-A alt-B}
    );

    assert_eq!(
        vec![
            Key(Modifiers::empty(), KeyCode::A),
            Key(Modifiers::empty(), KeyCode::B)
        ],
        keyseq! { A B}
    );
}

#[test]
fn test_key_eq_not() {
    let a: Key = KeyCode::A.into();
    let b: Key = KeyCode::B.into();
    // assert_eq!(a, b);
    assert!(a != b);
}

#[test]
fn test_key_eq_vec() {
    let a: Vec<Key> = vec![KeyCode::A.into()];
    let b: Vec<Key> = vec![KeyCode::B.into()];
    let c: Vec<Key> = vec![KeyCode::A.into()];
    let e: Vec<Key> = vec![];
    assert!(a != b);
    assert!(a == c);
    assert_eq!(a, c);
    assert!(e != a);
    assert!(e != b);
    assert!(e != c);
}
