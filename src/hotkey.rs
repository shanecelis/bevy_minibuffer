use bevy::prelude::*;
use bitflags::bitflags;
use crate::commands::*;

pub fn hotkey_input(
    mut run_command: EventWriter<RunCommandEvent>,
    keys: Res<Input<KeyCode>>,
    mut config: ResMut<CommandConfig>,
    mut last_keys: Local<Vec<Key>>,
) {
    let mods = Modifiers::from_input(&keys);
    let trie = config.hotkeys();
    let mut matches = vec![];

    for key_code in keys.get_just_pressed() {
        let key = Key::new(key_code.clone(), mods);
        last_keys.push(key);
        eprintln!("key seq {:?}", *last_keys);
        if (trie.exact_match(&*last_keys)) {
            eprintln!("got match {:?}", last_keys);
            let mut new_keys = vec![];
            std::mem::swap(&mut new_keys, &mut *last_keys);
            matches.push(new_keys);
            // Let's assume it's for running a command
            // last_keys.clear();
        } else if (trie.predictive_search(&*last_keys).is_empty()) {
            eprintln!("No key seq prefix for {:?}", *last_keys);
            last_keys.clear();
        }
    }

    for amatch in matches.into_iter() {
        for command in &config.commands {
            if let Some(ref keyseq) = command.hotkey {
                eprintln!("Comparing against command {:?}", keyseq);
                if &amatch == keyseq {
                // if hotkey.mods == mods && keys.just_pressed(hotkey.key) {
                    eprintln!("We were called for {}", command.name);

                    run_command.send(RunCommandEvent(Box::new(CommandOneShot(
                        command.name.clone(),
                    ))))
                }
            }
        }
    }
}

bitflags! {
    #[derive(Clone, Copy, Debug, PartialOrd, PartialEq, Eq, Hash, Ord)]
    pub struct Modifiers: u8 {
        const Alt     = 0b00000001;
        const Control = 0b00000010;
        const Shift   = 0b00000100;
        const System  = 0b00001000; // Windows or Command
    }
}

// alt-ctrl-shift-KeyCode::A
// m::alt | m::ctrl

lalrpop_mod!(pub key); // synthesized by LALRPOP

#[derive(Debug, Clone, PartialOrd, PartialEq, Eq, Hash, Ord)]
pub struct Key {
    pub mods: Modifiers,
    pub key: KeyCode,
}

pub type KeySeq = Vec<Key>;

impl Key {
    fn new(v: KeyCode, mods: Modifiers) -> Self {
        Key {
            key: v,
            mods
        }
    }
}

impl From<KeyCode> for Key {
    fn from(v: KeyCode) -> Self {
        Key {
            key: v,
            mods: Modifiers::empty(),
        }
    }
}

impl Modifiers {
    fn from_input(input: &Res<Input<KeyCode>>) -> Modifiers {
        let mut mods = Modifiers::empty();
        if input.any_pressed([KeyCode::LShift, KeyCode::RShift]) {
            mods |= Modifiers::Shift;
        }
        if input.any_pressed([KeyCode::LControl, KeyCode::RControl]) {
            mods |= Modifiers::Control;
        }
        if input.any_pressed([KeyCode::LAlt, KeyCode::RAlt]) {
            mods |= Modifiers::Alt;
        }
        if input.any_pressed([KeyCode::LWin, KeyCode::RWin]) {
            mods |= Modifiers::System;
        }
        mods
    }
}

macro_rules! keybind {

  // ( $key:ident) => {
  //     Key::from(keybind!(@key $key))
  // };
  //
    ($($($mods:ident)|+ -$key:tt),+) => {
        {
        let mut v = vec![];
        $(
            v.push(keybind!(@key $($mods)|+ - $key));
        )+
        v
        }
    };

    (@key $($mods:ident)|+ -$key:tt) => {
        {
        let mut accum = Modifiers::empty();
        $(
            accum |= keybind!(@lookup $mods);
        )*
        Key::new(keybind!(@keycode $key), accum)
        }
    };
    (@keycode ;) => { KeyCode::Semicolon };
    (@keycode :) => { KeyCode::Colon };
    (@keycode /) => { KeyCode::Slash };

    (@keycode $key:tt) => {
       paste::paste! { KeyCode::[<$key:camel>] }
    };

    (@lookup ctrl) => { Modifiers::Control };
    (@lookup alt) => { Modifiers::Alt };
    (@lookup sys) => { Modifiers::System };
    (@lookup shift) => { Modifiers::Shift };
}


#[cfg(test)]
mod tests {

    use bevy::prelude::*;
    use crate::commands::*;
    use crate::hotkey::*;
    use nano_macro::*;
    #[allow(unused_must_use)]
    #[test]
    fn test_key_eq() {
        let a: Key = KeyCode::A.into();
        let b: Key = KeyCode::A.into();
        assert_eq!(a, b);
        assert!(a == b);
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

        noop! { a-'\'' };
        // assert_eq!(Modifiers::Control | Modifiers::Alt, key!{ ctrl-alt });
        // assert_eq!(Modifiers::Control | Modifiers::Alt, key!{ ctrl - alt });
        // assert_eq!(Modifiers::Control | Modifiers::Alt, key!{ ctrl - alt });
    }

    #[allow(unused_must_use)]
    #[test]
    fn test_keybind() {
        assert_eq!(vec![Key::new(KeyCode::A, Modifiers::Control)], keybind!{ ctrl-A });
        assert_eq!(vec![Key::new(KeyCode::A, Modifiers::Control)], keybind!{ ctrl-a });
        assert_eq!(vec![Key::new(KeyCode::A, Modifiers::Control)], keybind!{ ctrl|ctrl-A });
        assert_eq!(vec![Key::new(KeyCode::A, Modifiers::Control),
                   Key::new(KeyCode::B, Modifiers::Alt)], keybind!{ ctrl-A, alt-B});

        // XXX: These don't work. Ugh.
        // assert_eq!(vec![Key::new(KeyCode::A, Modifiers::empty()),
        //            Key::new(KeyCode::B, Modifiers::empty())], keybind!{ A, B});
        // assert_eq!(Key::new(KeyCode::A, Modifiers::Control | Modifiers::Alt), keybind!{ ctrl|alt-A });
        // assert_eq!(Key::new(KeyCode::A, Modifiers::Control | Modifiers::Alt), keybind!{ ctrl|alt-a });
        // assert_eq!(Key::new(KeyCode::Semicolon, Modifiers::Control | Modifiers::Alt), keybind!{ ctrl|alt-semicolon });
        // assert_eq!(Key::new(KeyCode::Semicolon, Modifiers::Control | Modifiers::Alt), keybind!{ ctrl|alt-; });
        // assert_eq!(Key::new(KeyCode::Colon, Modifiers::Control | Modifiers::Alt), keybind!{ ctrl|alt-: });
        // assert_eq!(Key::new(KeyCode::Slash, Modifiers::Control | Modifiers::Alt), keybind!{ ctrl|alt-/ });
        // assert_eq!(Modifiers::Control | Modifiers::Alt, keybind!{ ctrl|alt});
        // assert_eq!(Modifiers::Control | Modifiers::Alt, keybind!{ ctrl | alt});
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

    use crate::hotkey::*;
    use crate::hotkey::key::*;
    #[allow(unused_must_use)]
    #[test]
    fn test_parser_ctrl() {
        assert!(ModsParser::new().parse("ctrl").is_ok());
        assert!(ModsParser::new().parse(" ctrl").is_err());
        assert!(ModsParser::new().parse("f ctrl").is_err());
        assert_eq!(Modifiers::Control, ModsParser::new().parse("ctrl").unwrap());
        assert_eq!(Modifiers::Control, ModsParser::new().parse("ctrl").unwrap());
        assert_eq!(Modifiers::Control | Modifiers::Alt, ModsParser::new().parse("ctrl-alt").unwrap());
        assert!(ModsParser::new().parse(" ctrl - alt").is_err());
    }

    // use serde::{Serialize, Deserialize};
    #[test]
    fn test_serde_keycode() {
        assert_eq!("Key1", format!("{:?}", KeyCode::Key1));
    }
}
