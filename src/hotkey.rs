use crate::commands::*;
use bevy::prelude::*;

pub use keyseq::{Modifiers, bevy::{pkey as key, pkeyseq as keyseq}};

// pub fn hotkey_input(
//     mut run_command: EventWriter<RunCommandEvent>,
//     keys: Res<ButtonInput<KeyCode>>,
//     mut config: ResMut<CommandConfig>,
//     mut last_keys: Local<Vec<Key>>,
// ) {
//     let mods = Modifiers::from_input(&keys);
//     let trie = config.hotkeys();
//     let mut matches = vec![];

//     for key_code in keys.get_just_pressed() {
//         let key = (mods, *key_code);
//         last_keys.push(key);
//         // eprintln!("key seq {:?}", *last_keys);
//         if trie.exact_match(&*last_keys) {
//             // eprintln!("got match {:?}", last_keys);
//             let mut new_keys = vec![];
//             std::mem::swap(&mut new_keys, &mut *last_keys);
//             matches.push(new_keys);
//             // Let's assume it's for running a command
//             // last_keys.clear();
//         } else if trie.predictive_search(&*last_keys).is_empty() {
//             // eprintln!("No key seq prefix for {:?}", *last_keys);
//             last_keys.clear();
//         }
//     }

//     for amatch in matches.into_iter() {
//         for command in &config.commands {
//             if let Some(ref keyseq) = command.hotkey {
//                 // eprintln!("Comparing against command {:?}", keyseq);
//                 if &amatch == keyseq {
//                     // if hotkey.mods == mods && keys.just_pressed(hotkey.key) {
//                     // eprintln!("We were called for {}", command.name);
//                     run_command.send(RunCommandEvent(command.system_id.expect("No system_id for command.")));
//                 }
//             }
//         }
//     }
// }

// Consider using arrayvec::ArrayVec instead of Vec since key sequences will
// rarely go over 5. A Vec occupies 24 bytes on 64-bit machines on the stack or
// 192 bits. A KeyCode is 32 bits. A Key is Modifiers + KeyCode or 8 + 32 = 40
// bits. So instead of having a Vec on the stack and its contents on the heap,
// we could have 192 bits/40 bits = 4.8 Keys for the same stack price.
pub type KeySeq = Vec<Key>;
pub type Key = (Modifiers, KeyCode);
