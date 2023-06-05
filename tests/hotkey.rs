
// extern crate bevy;
// #[cfg(test)]
use nano_macro::*;
use bevy::prelude::*;
use bevy_nano_console::hotkey::*;
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
        // assert_eq!(Key::new(KeyCode::A, Modifiers::Control),
    }

    #[allow(unused_must_use)]
    #[test]
    fn test_keyseq_macro() {
        assert_eq!(vec![Key::new(KeyCode::A, Modifiers::empty())], keyseq!{ A });
        assert_eq!(vec![Key::new(KeyCode::A, Modifiers::empty()),
                        Key::new(KeyCode::B, Modifiers::empty()),
        ], keyseq!{ A B });
    }

    #[allow(unused_must_use)]
    #[test]
    fn test_key_macro() {
        assert_eq!(Key::new(KeyCode::A, Modifiers::Control), key!{ ctrl-A });
        assert_eq!(Key::new(KeyCode::A, Modifiers::Control), key!{ ctrl-a });
        assert_eq!(Key::new(KeyCode::B, Modifiers::Control), key!{ ctrl-b });
        assert_eq!(Key::new(KeyCode::Key1, Modifiers::Control), key!{ ctrl-1 });
        assert_eq!(Key::new(KeyCode::Key2, Modifiers::Control), key!{ ctrl-2 });
        assert_eq!(Key::new(KeyCode::F2, Modifiers::Control), key!{ ctrl-F2 });
        // assert_eq!(Key::new(KeyCode::F2, Modifiers::Control), key!{ ctrl-f2 });
        assert_eq!(Key::new(KeyCode::Semicolon, Modifiers::Control), key!{ ctrl-; });
        assert_eq!(Key::new(KeyCode::Caret, Modifiers::Control), key!{ ctrl-^ });
        assert_eq!(Key::new(KeyCode::Colon, Modifiers::Control), key!{ ctrl-: });
        assert_eq!(Key::new(KeyCode::Equals, Modifiers::Control), key!{ ctrl-= });
        assert_eq!(Key::new(KeyCode::Comma, Modifiers::Control), key!{ ctrl-, });
        assert_eq!(Key::new(KeyCode::Period, Modifiers::Control), key!{ ctrl-. });
        assert_eq!(Key::new(KeyCode::Slash, Modifiers::Control), key!{ ctrl-/ });
        assert_eq!(Key::new(KeyCode::Minus, Modifiers::Control), key!{ ctrl-- });
        assert_eq!(Key::new(KeyCode::Underline, Modifiers::Control), key!{ ctrl-_ });
        assert_eq!(Key::new(KeyCode::Asterisk, Modifiers::Control), key!{ ctrl-* });
        assert_eq!(Key::new(KeyCode::Plus, Modifiers::Control), key!{ ctrl-+ });
        assert_eq!(Key::new(KeyCode::At, Modifiers::Control), key!{ ctrl-@ });
        assert_eq!(Key::new(KeyCode::Grave, Modifiers::Control), key!{ ctrl-'`' });
        assert_eq!(Key::new(KeyCode::Backslash, Modifiers::Control), key!{ ctrl-'\\' });
        assert_eq!(Key::new(KeyCode::Escape, Modifiers::Control), key!{ ctrl-Escape });
        // assert_eq!(Key::new(KeyCode::Escape, Modifiers::Control), key!{ ctrl-Esc });
        assert_eq!(Key::new(KeyCode::A, Modifiers::Control | Modifiers::Alt), key!{ ctrl-alt-A });
        assert_eq!(Key::new(KeyCode::A, Modifiers::empty()), key!{ A });
        assert_eq!(Key::new(KeyCode::A, Modifiers::Control | Modifiers::Alt), key!{ ctrl-alt-A });
        assert_eq!(Key::new(KeyCode::A, Modifiers::Control | Modifiers::Alt), key!{ ctrl-alt-a });
        assert_eq!(Key::new(KeyCode::Semicolon, Modifiers::Control | Modifiers::Alt), key!{ ctrl-alt-Semicolon });
        // assert_eq!(Key::new(KeyCode::Semicolon, Modifiers::Control | Modifiers::Alt), key!{ ctrl-alt-semicolon });
        assert_eq!(Key::new(KeyCode::Semicolon, Modifiers::Control | Modifiers::Alt), key!{ ctrl-alt-; });
        assert_eq!(Key::new(KeyCode::Colon, Modifiers::Control | Modifiers::Alt), key!{ ctrl-alt-: });
        assert_eq!(Key::new(KeyCode::Slash, Modifiers::Control | Modifiers::Alt), key!{ ctrl-alt-/ });

        // assert_eq!(Modifiers::Control | Modifiers::Alt, key!{ ctrl-alt });
        // assert_eq!(Modifiers::Control | Modifiers::Alt, key!{ ctrl - alt });
        // assert_eq!(Modifiers::Control | Modifiers::Alt, key!{ ctrl - alt });
    }

    #[allow(unused_must_use)]
    #[test]
    fn test_keyseq() {
        assert_eq!(vec![Key::new(KeyCode::A, Modifiers::Control)], keyseq!{ ctrl-A });
        assert_eq!(vec![Key::new(KeyCode::A, Modifiers::Control)], keyseq!{ ctrl-a });
        assert_eq!(vec![Key::new(KeyCode::A, Modifiers::Control)], keyseq!{ ctrl-ctrl-A });
        assert_eq!(vec![Key::new(KeyCode::A, Modifiers::Control),
                   Key::new(KeyCode::B, Modifiers::Alt)], keyseq!{ ctrl-A alt-B});

        assert_eq!(vec![Key::new(KeyCode::A, Modifiers::empty()),
                   Key::new(KeyCode::B, Modifiers::empty())], keyseq!{ A B});
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

