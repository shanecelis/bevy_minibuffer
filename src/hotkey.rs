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

  ( $($mods:ident)|* -$key:tt) => {
      {
      // let ctrl = Modifiers::Control;
      let mut accum = Modifiers::empty();
      $(
          accum |= keybind!(@lookup $mods);
      )*
      Key::new(keybind!(@key $key), accum)
      }
  };
    (@key ;) => { KeyCode::Semicolon };
    (@key :) => { KeyCode::Colon };
    (@key /) => { KeyCode::Slash };

    (@key $key:tt) => {
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
    fn test_keybind() {
        assert_eq!(Key::new(KeyCode::A, Modifiers::Control), keybind!{ ctrl-A });
        assert_eq!(Key::new(KeyCode::A, Modifiers::Control | Modifiers::Alt), keybind!{ ctrl|alt-A });
        assert_eq!(Key::new(KeyCode::A, Modifiers::Control | Modifiers::Alt), keybind!{ ctrl|alt-a });
        assert_eq!(Key::new(KeyCode::Semicolon, Modifiers::Control | Modifiers::Alt), keybind!{ ctrl|alt-semicolon });
        assert_eq!(Key::new(KeyCode::Semicolon, Modifiers::Control | Modifiers::Alt), keybind!{ ctrl|alt-; });
        assert_eq!(Key::new(KeyCode::Colon, Modifiers::Control | Modifiers::Alt), keybind!{ ctrl|alt-: });
        assert_eq!(Key::new(KeyCode::Slash, Modifiers::Control | Modifiers::Alt), keybind!{ ctrl|alt-/ });
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
