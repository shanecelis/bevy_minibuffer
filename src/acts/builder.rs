//! Acts and their flags, builders, and collections
use crate::{
    acts::{Act, ActFlags, ActWithInputSystem, RunActMap},
    input::Hotkey,
    ui::ActContainer,
};
use bevy::{ecs::system::EntityCommand, prelude::*};
use bevy_input_sequence::KeyChord;
use std::{
    any::TypeId,
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
    system_name: Cow<'static, str>,
    register_system: Box<dyn FnOnce(&mut World) -> Entity + 'static + Send + Sync>,
    // pub(crate) system: Option<BoxedSystem>,
    /// Flags for this act
    pub flags: ActFlags,
    /// Shorten the name to just the first system.
    pub shorten_name: bool,
    input: Option<TypeId>,
}

impl fmt::Debug for ActBuilder {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("ActBuilder")
            .field("name", &self.name)
            .field("hotkeys", &self.hotkeys)
            .field(
                "register_system",
                &"Box<dyn FnOnce(&mut World) -> Entity { ... }",
            )
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
        let system_name = system.name();
        let name = Self::name_for_system(&system, true);
        ActBuilder {
            name,
            system_name,
            register_system: Box::new(move |world: &mut World| {
                let system_id = world.register_system(system);
                system_id.entity()
            }),
            hotkeys: Vec::new(),
            flags: ActFlags::default(),
            shorten_name: true,
            input: None,
        }
    }

    pub fn new_with_input<S, I, P>(system: S) -> Self
    where
        S: IntoSystem<In<I>, (), P> + 'static,
        I: 'static + Default + Clone + Send + Sync + Debug,
    {
        let system = IntoSystem::into_system(system);
        let system_name = system.name();
        let name = Self::name_for_system(&system, true);
        ActBuilder {
            name,
            system_name,
            register_system: Box::new(move |world: &mut World| {
                let mut run_act_map = world.resource_mut::<RunActMap>();
                let type_id = TypeId::of::<I>();
                run_act_map
                    .entry(type_id)
                    .or_insert_with(|| Box::new(ActWithInputSystem::<I>::new()));
                let system_id = world.register_system(system);
                system_id.entity()
            }),
            hotkeys: Vec::new(),
            flags: ActFlags::default(),
            shorten_name: true,
            input: Some(TypeId::of::<I>()),
        }
    }

    fn name_for_system<S: System>(system: &S, shorten_name: bool) -> Cow<'static, str> {
        let mut n = system.name();
        // Take name out of pipe.
        //
        // "Pipe(cube_async::speed, bevy_minibuffer::sink::future_result<(), bevy_minibuffer::plugin::Error, cube_async::speed::{{closure}}>)"
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
    pub fn build(self, world: &mut World) -> (Act, Entity) {
        let name = self.name;
        // let id = (self.make_act_runner)(world);
        let system_id = (self.register_system)(world);
        // let id = system_id.entity();
        // world.get_entity_mut(id).expect("entity for system_id")
        //     .insert(RunActMap::new(ActSystem(system_id)));
        (
            Act {
                name,
                hotkeys: self.hotkeys,
                flags: self.flags,
                system_id,
                system_name: self.system_name,
                input: self.input,
            },
            system_id,
        )
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
        let taken: Cow<'static, str> = "*TAKEN*".into();
        Self {
            name: std::mem::replace(&mut builder.name, taken.clone()),
            register_system: std::mem::replace(
                &mut builder.register_system,
                Box::new(|_world: &mut World| {
                    warn!("Tried to register a depleted ActBuilder.");
                    Entity::PLACEHOLDER
                }),
            ),
            // system: builder.system.take(),
            hotkeys: std::mem::take(&mut builder.hotkeys),
            flags: builder.flags,
            shorten_name: builder.shorten_name,
            system_name: std::mem::replace(&mut builder.system_name, taken),
            input: builder.input.take(),
        }
    }
}

impl Command for ActBuilder {
    fn apply(self, world: &mut World) {
        let (act, id) = self.build(world);
        let name = Name::new(act.name.clone());
        let keyseqs = act.build_keyseqs(id, world);
        world.entity_mut(id).insert(act).insert(name);

        // let id = world.spawn(act).insert(name).id();
        for keyseq_id in keyseqs {
            world.entity_mut(keyseq_id).insert(ChildOf(id));
        }
        let mut query = world.query_filtered::<Entity, With<ActContainer>>();
        if let Ok(act_container) = query.single(world) {
            world.entity_mut(id).insert(ChildOf(act_container));
        }
    }
}

impl EntityCommand for ActBuilder {
    fn apply(self, mut entity_world: EntityWorldMut) {
        let id = entity_world.id();

        entity_world.world_scope(move |world: &mut World| {
            let (act, system_id) = self.build(world);
            let keyseqs = act.build_keyseqs(id, world);
            let mut entity = world.get_entity_mut(id).unwrap();
            entity.insert(act);
            for keyseq_id in keyseqs {
                world.entity_mut(keyseq_id).insert(ChildOf(id));
            }
            world.entity_mut(system_id).insert(ChildOf(id));
        });
    }
}
