use bevy::prelude::*;

pub use keyseq::{Modifiers, bevy::{pkey as key, pkeyseq as keyseq}};

// Consider using arrayvec::ArrayVec instead of Vec since key sequences will
// rarely go over 5. A Vec occupies 24 bytes on 64-bit machines on the stack or
// 192 bits. A KeyCode is 32 bits. A Key is Modifiers + KeyCode or 8 + 32 = 40
// bits. So instead of having a Vec on the stack and its contents on the heap,
// we could have 192 bits/40 bits = 4.8 Keys for the same stack price.
pub type KeySeq = Vec<Key>;
pub type Key = (Modifiers, KeyCode);
