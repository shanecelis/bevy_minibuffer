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
        let key = Key(mods, key_code.clone());
        last_keys.push(key);
        eprintln!("key seq {:?}", *last_keys);
        if trie.exact_match(&*last_keys) {
            eprintln!("got match {:?}", last_keys);
            let mut new_keys = vec![];
            std::mem::swap(&mut new_keys, &mut *last_keys);
            matches.push(new_keys);
            // Let's assume it's for running a command
            // last_keys.clear();
        } else if trie.predictive_search(&*last_keys).is_empty() {
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


#[derive(Debug, Clone, PartialOrd, PartialEq, Eq, Hash, Ord)]
pub struct Key (pub Modifiers, pub KeyCode);

pub type KeySeq = Vec<Key>;

/// ```
/// use nano_macro::key;
///    key!{ ctrl-A };
/// ```
///
/// ```compile_fail
/// use nano_macro::key;
///    key!{ ctrl-A b };
/// ```
// impl Key {
//     pub fn new(v: KeyCode, mods: Modifiers) -> Self {
//         Key {
//             key: v,
//             mods
//         }
//     }
// }

impl From<KeyCode> for Key {
    fn from(v: KeyCode) -> Self {
        Key(Modifiers::empty(), v)
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
