use crate::{
    autocomplete::LookupError,
    event::{dispatch_events, run_acts, run_acts_by_name, KeyChordEvent, LookupEvent},
    prompt::{
        self, get_key_chords, hide, hide_delayed, hide_prompt_maybe, listen_prompt_active,
        lookup_events, show, CompletionState, MinibufferState, PromptState,
    },
    ui,
};
use bevy::{
    app::{PluginGroupBuilder, Update},
    ecs::{
        schedule::{
            IntoSystemSetConfigs,
            SystemSet,
            // on_event,
        },
        system::Resource,
    },
    prelude::{IntoSystemConfigs, OnEnter, OnExit, PluginGroup},
    reflect::Reflect,
    state::{
        app::AppExtStates,
        // OnEnter, OnExit,
        condition::in_state,
    },
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
    /// An Asky error
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
struct InputSequenceSet;

#[rustfmt::skip]
impl bevy::app::Plugin for MinibufferPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app
            .register_type::<PromptState>()
            .register_type::<CompletionState>()
            .register_type::<Config>()
            .add_plugins(crate::ui::plugin)
            .add_plugins(crate::event::plugin)
            .add_plugins(crate::prompt::plugin)
            .add_plugins(crate::autocomplete::plugin)
            .add_plugins(crate::view::plugin)
            .add_plugins(crate::acts::plugin)
            .add_plugins(AskyPlugin)
            .add_plugins(InputSequencePlugin::empty()
                         .match_button(false)
                         .run_in_set(Update, InputSequenceSet))
            .init_state::<MinibufferState>()
            .init_state::<PromptState>()
            .init_state::<CompletionState>()
            .insert_resource(self.config.clone())
            .add_event::<LookupEvent>()
            .add_event::<KeyChordEvent>()
            .add_systems(Update,
                         (hide_prompt_maybe,
                          // acts::detect_additions,
                          //asky::bevy::asky_system::<AutoComplete<asky::Text>>,
                          listen_prompt_active)
                         .in_set(MinibufferSet::Process))
            .add_systems(Update, get_key_chords.in_set(MinibufferSet::Input))
            .configure_sets(Update, (
                (MinibufferSet::Input, MinibufferSet::Process, MinibufferSet::Output).chain(),
                InputSequenceSet.after(MinibufferSet::Input),
                InputSequenceSet.run_if(in_state(MinibufferState::Inactive)),
            ))
            .add_systems(Update,
                         ((run_acts_by_name, run_acts, prompt::set_minibuffer_state).chain(),
                          (dispatch_events, lookup_events).chain())
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
