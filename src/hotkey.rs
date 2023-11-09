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
        let key = Key(mods, *key_code);
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
                    run_command.send(RunCommandEvent(command.system_id.unwrap()));
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
        const Super   = 0b00001000; // Windows or Command
    }
}

// alt-ctrl-shift-KeyCode::A
// m::alt | m::ctrl


#[derive(Debug, Clone, PartialOrd, PartialEq, Eq, Hash, Ord)]
pub struct Key (pub Modifiers, pub KeyCode);

// Consider using arrayvec::ArrayVec instead of Vec since key sequences will
// rarely go over 5. A Vec occupies 24 bytes on 64-bit machines on the stack or
// 192 bits. A KeyCode is 32 bits. A Key is Modifiers + KeyCode or 8 + 32 = 40
// bits. So instead of having a Vec on the stack and its contents on the heap,
// we could have 192 bits/40 bits = 4.8 Keys for the same stack price.
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
        if input.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]) {
            mods |= Modifiers::Shift;
        }
        if input.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]) {
            mods |= Modifiers::Control;
        }
        if input.any_pressed([KeyCode::AltLeft, KeyCode::AltRight]) {
            mods |= Modifiers::Alt;
        }
        if input.any_pressed([KeyCode::SuperLeft, KeyCode::SuperRight]) {
            mods |= Modifiers::Super;
        }
        mods
    }
}
