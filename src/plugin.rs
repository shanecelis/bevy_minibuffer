use crate::{
    act,
    event::{dispatch_events, run_acts, LookupEvent, RunActEvent, RunInputSequenceEvent},
    lookup::LookupError,
    prompt::{
        self, get_key_chords, hide, hide_delayed, hide_prompt_maybe, listen_prompt_active,
        look_up_events, show, CompletionState, KeyChordEvent, MinibufferState, PromptState,
    },
    ui,
};
use bevy::{
    app::{PluginGroupBuilder, Update},
    ecs::{
        schedule::{
            Condition,
            IntoSystemSetConfigs,
            SystemSet,
            // on_event,
        },
        system::Resource,
    },
    prelude::{Deref, DerefMut, IntoSystemConfigs, Event, on_event, OnEnter, OnExit, PluginGroup},
    reflect::Reflect,
    state::{
        app::AppExtStates,
        // OnEnter, OnExit,
        condition::in_state,
    },
    text::TextStyle,
};
use bevy_asky::AskyPlugin;
use bevy_input_sequence::InputSequencePlugin;
use std::{borrow::Cow, time::Duration};

/// Minibuffer plugin
#[derive(Debug, Default, Clone)]
pub struct MinibufferPlugin {
    /// Configuration
    pub config: Config,
}

/// Minibuffer plugins, includes [bevy_defer::AsyncPlugin] with default settings
/// if "async" feature is present.
pub struct MinibufferPlugins;

impl PluginGroup for MinibufferPlugins {
    fn build(self) -> PluginGroupBuilder {
        let group = PluginGroupBuilder::start::<Self>().add(MinibufferPlugin::default());
        #[cfg(feature = "async")]
        // let group = group.add(bevy_defer::AsyncPlugin::default_settings());
        let group = group.add(bevy_defer::AsyncPlugin::empty().run_in(bevy::prelude::PreUpdate));
        group
    }
}

/// Minibuffer config
#[derive(Debug, Resource, Clone, Default, Reflect)]
pub struct Config {
    /// If true, auto hide minibuffer after use.
    pub auto_hide: bool,
    /// Auto hide delay.
    pub hide_delay: Duration,
    /// The text style for minibuffer
    pub text_style: TextStyle,
}

/// When we resolve a string, it can be mapped to another value T.
#[derive(Event, Deref, DerefMut, Debug)]
pub struct Mapped<T> {
    /// The result if not taken yet.
    #[deref]
    pub result: Option<Result<T, Error>>,
    /// Input string mapped from if available.
    pub input: Option<String>,
}

impl<T> Mapped<T> {
    /// Create a new mapped event.
    pub fn new(result: Result<T, Error>) -> Self {
        Self {
            result: Some(result),
            input: None
        }
    }

    /// Create an empty mapped event.
    pub fn empty() -> Self {
        Self {
            result: None,
            input: None
        }
    }

    /// Provide input string if available.
    pub fn with_input(mut self, input: String) -> Self {
        self.input = Some(input);
        self
    }

    /// Unwrap the result assuming it hasn't been taken already.
    pub fn take_result(&mut self) -> Result<T, Error> {
        self.result.take().expect("mapped has been taken already")
    }
}

/// Minibuffer error
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An error message
    #[error("{0}")]
    Message(Cow<'static, str>),
    /// A lookup error
    #[error("look up error{0}")]
    Lookup(#[from] LookupError),
    /// An [asky] error
    #[error("{0}")]
    Asky(#[from] bevy_asky::Error),
    /// An async error
    #[cfg(feature = "async")]
    #[error("async error {0}")]
    Async(#[from] bevy_defer::AccessError),

    /// A futures error
    #[cfg(feature = "async")]
    #[error("future error {0}")]
    Futures(#[from] futures::channel::oneshot::Canceled),
}

/// Minibuffer generally runs in the Update schedule of sets in this order where
/// necessary: Input, Process, Output.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum MinibufferSet {
    Input,
    Process,
    Output,
}

/// This is a separate system set because it is toggled on and off depending on
/// wheter we want key sequences to be detected or not.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
struct InputSet;

#[rustfmt::skip]
impl bevy::app::Plugin for MinibufferPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app
            .register_type::<PromptState>()
            .register_type::<CompletionState>()
            .register_type::<Config>()
            .register_type::<act::Act>()
            .add_plugins(crate::ui::plugin)
            .add_plugins(crate::event::plugin)
            .add_plugins(crate::prompt::plugin)
            .add_plugins(crate::autocomplete::plugin)
            .add_plugins(crate::view::plugin)
            .add_plugins(AskyPlugin)
            .add_plugins(InputSequencePlugin::empty().run_in_set(Update, InputSet))
            .init_state::<MinibufferState>()
            .init_state::<PromptState>()
            .init_state::<CompletionState>()
            .init_resource::<act::ActCache>()
            .insert_resource(self.config.clone())
            .add_event::<RunInputSequenceEvent>()
            .add_event::<LookupEvent>()
            .add_event::<RunActEvent>()
            .add_event::<KeyChordEvent>()
            .add_systems(Update,
                         (hide_prompt_maybe,
                          // act::detect_additions,
                          //asky::bevy::asky_system::<AutoComplete<asky::Text>>,
                          listen_prompt_active)
                         .in_set(MinibufferSet::Process))
            .add_systems(Update, get_key_chords.in_set(MinibufferSet::Input))
            .configure_sets(Update, (
                (MinibufferSet::Input, MinibufferSet::Process, MinibufferSet::Output).chain(),
                InputSet.after(MinibufferSet::Input),
                InputSet.run_if(in_state(MinibufferState::Inactive).or_else(on_event::<RunInputSequenceEvent>())),
            ))
            .observe(crate::event::dispatch_trigger)
            .add_systems(Update,
                         ((run_acts, prompt::set_minibuffer_state).chain(),
                          (dispatch_events, look_up_events).chain())
                         .in_set(MinibufferSet::Process))
            .add_systems(OnEnter(MinibufferState::Inactive),hide_delayed::<ui::PromptContainer>)
            .add_systems(OnEnter(MinibufferState::Inactive),hide::<ui::CompletionContainer>)
            .add_systems(OnEnter(PromptState::Visible),     show::<ui::PromptContainer>)
            .add_systems(OnEnter(PromptState::Invisible),   hide::<ui::PromptContainer>)
            .add_systems(OnEnter(CompletionState::Visible), show::<ui::CompletionContainer>)
            .add_systems(OnExit(CompletionState::Visible),  hide::<ui::CompletionContainer>)
            ;
    }
}
