//! Acts and their flags, builders, and collections
use crate::{
    acts::{Act, ActFlags, ActSystem, ActWithInputSystem, ActRunner},
    input::Hotkey,
};
use bevy::{
    ecs::{
        system::{BoxedSystem, EntityCommand},
        world::Command,
    },
    prelude::*,
};
use bevy_input_sequence::KeyChord;
use std::{
    borrow::Cow,
    fmt::{
        self,
        Debug,
        // Write
    },
};

/// Builds an [Act]
// #[derive(Debug)]
pub struct ActBuilder {
    pub name: Cow<'static, str>,
    /// Hotkeys
    pub hotkeys: Vec<Hotkey>,
    make_act_runner: Box<dyn FnOnce(&mut World) -> Entity + 'static + Send + Sync>,
    // pub(crate) system: Option<BoxedSystem>,
    /// Flags for this act
    pub flags: ActFlags,
    /// Shorten the name to just the first system.
    pub shorten_name: bool,
}

impl fmt::Debug for ActBuilder {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("ActBuilder")
            .field("name", &self.name)
            .field("hotkeys", &self.hotkeys)
            .field("make_act_runner", &"Box<dyn FnOnce(&mut World) -> Entity { ... }")
            .field("flags", &self.flags)
            .field("shorten_name", &self.shorten_name)
            .finish()
    }
}

impl ActBuilder {
    /// Create a new [Act].
    pub fn new<S, P>(system: S) -> Self
    where
        S: IntoSystem<(), (), P> + 'static,
    {
        let system = IntoSystem::into_system(system);
        let name = Self::name_for_system(&system, true);
        let make_act_runner = Box::new(move |world: &mut World| {
            let system_id = world.register_system(system);
            let id = system_id.entity();
            world.get_entity_mut(id).expect("entity for system_id")
                                    .insert(ActRunner::new(ActSystem(system_id)));
            id
        });
        ActBuilder {
            name,
            hotkeys: Vec::new(),
            make_act_runner,
            flags: ActFlags::default(),
            shorten_name: true,
        }
    }

    pub fn new_with_input<S, I, P>(system: S) -> Self
        where S: IntoSystem<In<I>,(), P> + 'static,
    I: 'static + Default + Clone + Send + Sync
    {
        let system = IntoSystem::into_system(system);
        let name = Self::name_for_system(&system, true);
        let make_act_runner = Box::new(move |world: &mut World| {
            let system_id = world.register_system(system);
            let id = system_id.entity();
            world.get_entity_mut(id).expect("entity for system_id")
                                    .insert(ActRunner::new(ActWithInputSystem(system_id)));
            id
        });
        ActBuilder {
            name,
            hotkeys: Vec::new(),
            make_act_runner,
            flags: ActFlags::default(),
            shorten_name: true,
        }
    }

    // pub fn new_with_input<S, I, P>(system: S) -> Self
    //     where S: IntoSystem<In<Option<I>>,(), P> + 'static,
    // I: 'static
    //     {
    //     ActBuilder {
    //         name: None,
    //         hotkeys: Vec::new(),
    //         system: Some(Box::new(IntoSystem::into_system(system))),
    //         flags: ActFlags::Active | ActFlags::RunAct,
    //         shorten_name: true,
    //     }
    // }

    fn name_for_system<S: System>(system: &S, shorten_name: bool) -> Cow<'static, str> {
        let mut n = system.name();
        // Take name out of pipe.
        //
        // "Pipe(cube_async::speed, bevy_minibuffer::sink::future_result_sink<(), bevy_minibuffer::plugin::Error, cube_async::speed::{{closure}}>)"
        // -> "cube_async::speed"
        n = n
            .find('(')
            .and_then(|start| {
                n.find([',', ' ', ')'])
                    .map(|end| n[start + 1..end].to_owned().into())
            })
            .unwrap_or(n);
        if shorten_name {
            n = n
                .rfind(':')
                .map(|start| n[start + 1..].to_owned().into())
                .unwrap_or(n);
        }
        n
    }

    /// Return the name of the act. Derived from system if not explicitly given.
    pub fn name(&self) -> Cow<'static, str> {
        self.name.clone()
    }

    /// Build [Act].
    pub fn build(mut self, world: &mut World) -> Act {
        let name = self.name;
        let id = (self.make_act_runner)(world);
        // let system_id = world.register_boxed_system(self.system.take().expect("system"));
        // let id = system_id.entity();
        // world.get_entity_mut(id).expect("entity for system_id")
        //     .insert(ActRunner::new(ActSystem(system_id)));
        Act {
            name,
            hotkeys: self.hotkeys,
            flags: self.flags,
            system_id: id,
        }
    }

    /// Name the act.
    pub fn named(&mut self, name: impl Into<Cow<'static, str>>) -> &mut Self {
        self.name = name.into();
        self
    }

    /// Bind a hotkey.
    pub fn bind<T>(&mut self, hotkey: impl IntoIterator<Item = T>) -> &mut Self
    where
        KeyChord: From<T>,
    {
        self.hotkeys.push(Hotkey::new(hotkey));
        self
    }

    /// Bind a hotkey with an alias for that key sequence.
    ///
    /// ```no_compile
    /// // Bring comfort to Emacs users.
    /// act.bind_aliased(keyseq! { Alt-X }, "M-x");
    /// ```
    pub fn bind_aliased<T>(
        &mut self,
        hotkey: impl IntoIterator<Item = T>,
        name: impl Into<Cow<'static, str>>,
    ) -> &mut Self
    where
        KeyChord: From<T>,
    {
        self.hotkeys.push(Hotkey::new(hotkey).alias(name));
        self
    }

    /// Set flags.
    pub fn flags(&mut self, flags: ActFlags) -> &mut Self {
        self.flags = flags;
        self
    }

    /// Add the given the flags.
    pub fn add_flags(&mut self, flags: ActFlags) -> &mut Self {
        self.flags |= flags;
        self
    }

    /// Subtracts the given the flags.
    pub fn sub_flags(&mut self, flags: ActFlags) -> &mut Self {
        self.flags -= flags;
        self
    }
}

impl From<&mut ActBuilder> for ActBuilder {
    fn from(builder: &mut ActBuilder) -> Self {
        Self {
            name: std::mem::replace(&mut builder.name, "*TAKEN*".into()),
            make_act_runner: std::mem::replace(&mut builder.make_act_runner, Box::new(|world: &mut World| { Entity::PLACEHOLDER })),
            // system: builder.system.take(),
            hotkeys: std::mem::take(&mut builder.hotkeys),
            flags: builder.flags,
            shorten_name: builder.shorten_name,
        }
    }
}

impl Command for ActBuilder {
    fn apply(self, world: &mut World) {
        let act  = self.build(world);
        let keyseqs = act.build_keyseqs(world);
        let name = Name::new(act.name.clone());
        let system_entity = act.system_id;

        let id = world.spawn(act).insert(name).id();
        for keyseq_id in keyseqs {
            world.entity_mut(keyseq_id).set_parent(id);
        }
        world.entity_mut(system_entity).set_parent(id);
    }
}

impl EntityCommand for ActBuilder {
    fn apply(self, id: Entity, world: &mut World) {
        let act = self.build(world);
        let keyseqs = act.build_keyseqs(world);
        let mut entity = world.get_entity_mut(id).unwrap();
        entity.insert(act);
        for keyseq_id in keyseqs {
            world.entity_mut(keyseq_id).set_parent(id);
        }
    }
}
