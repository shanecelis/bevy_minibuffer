use crate::{
    act,
    event::{run_acts, DispatchEvent, LookUpEvent, RunActEvent, RunInputSequenceEvent, dispatch_events},
    // lookup::AutoComplete,
    prompt::{
        self, get_key_chords,
        hide, hide_delayed, hide_prompt_maybe,
        listen_prompt_active, look_up_events, show, CompletionState, MinibufferState, PromptState, KeyChordEvent
    },
    ui,
};
use bevy_asky::AskyPlugin;
use bevy::{
    app::{PostUpdate, Startup, Update, PluginGroupBuilder},
    state::{
        app::AppExtStates,
        // OnEnter, OnExit,
        condition::{in_state},
    },
    ecs::{
        reflect::AppTypeRegistry,
        schedule::{
            Condition, IntoSystemSetConfigs, SystemSet,
            // on_event,
        },
        system::Resource,
    },
    prelude::IntoSystemConfigs,
    reflect::Reflect,
    text::TextStyle,
    utils::default,
    prelude::{OnEnter, OnExit, on_event, PluginGroup}
};
use bevy_input_sequence::InputSequencePlugin;
use std::borrow::Cow;

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
        let group = PluginGroupBuilder::start::<Self>()
            .add(MinibufferPlugin::default());
        #[cfg(feature = "async")]
        let group = group
            .add(bevy_defer::AsyncPlugin::default_settings());
        group
    }
}

/// Minibuffer config
#[derive(Debug, Resource, Clone, Default, Reflect)]
pub struct Config {
    /// If true, auto hide minibuffer after use.
    pub auto_hide: bool,
    /// Auto hide delay in milliseconds.
    pub hide_delay: Option<u64>, // milliseconds
    /// The text style for minibuffer
    pub text_style: TextStyle,
}

// impl Default for ConsoleConfig {
//     fn default() -> Self {
//         Self {
//             hide_delay: Some(2000), /* milliseconds */
//         }
//     }
// }

/// Minibuffer error
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An error message
    #[error("{0}")]
    Message(Cow<'static, str>),
    /// An [asky] error
    #[error("asky {0}")]
    Asky(#[from] bevy_asky::Error),
    /// An async error
    #[cfg(feature = "async")]
    #[error("async error {0}")]
    Async(#[from] bevy_defer::AccessError),
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
enum MinibufferSet {
    Input,
    Process,
    Output,
}

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
            .add_plugins(crate::event::plugin)
            .add_plugins(crate::prompt::plugin)
            .add_plugins(crate::autocomplete::plugin)
            .add_plugins(AskyPlugin)
            .add_plugins(bevy_asky::view::color::plugin)
            .add_plugins(InputSequencePlugin::empty()
            .run_in_set(Update, InputSet))
            .init_state::<MinibufferState>()
            .init_state::<PromptState>()
            .init_state::<CompletionState>()
            .init_resource::<act::ActCache>()
            .insert_resource(self.config.clone())
            .add_event::<RunInputSequenceEvent>()
            .add_event::<LookUpEvent>()
            .add_event::<RunActEvent>()
            .add_event::<KeyChordEvent>()
            .add_systems(Startup, ui::spawn_layout)
            .add_systems(Update,
                         (hide_prompt_maybe,
                          // act::detect_additions,
                          //asky::bevy::asky_system::<AutoComplete<asky::Text>>,
                          listen_prompt_active)
                         .in_set(MinibufferSet::Process))
            .add_systems(Update, get_key_chords.in_set(MinibufferSet::Input))
            .configure_sets(Update,
                            (InputSet).after(MinibufferSet::Input))

            .configure_sets(Update, (
                // (MinibufferSet::Input, MinibufferSet::Process, MinibufferSet::Output).chain(),
                InputSet.after(MinibufferSet::Input),
                InputSet.run_if(in_state(MinibufferState::Inactive).or_else(on_event::<RunInputSequenceEvent>())),
            ))
            .add_systems(PostUpdate,
                         ((run_acts, prompt::set_minibuffer_state).chain(),
                          (dispatch_events, look_up_events).chain())
                         .in_set(MinibufferSet::Output))
            .add_systems(OnEnter(PromptState::Finished),    hide_delayed::<ui::PromptContainer>)
            .add_systems(OnEnter(PromptState::Visible),     show::<ui::PromptContainer>)
            .add_systems(OnEnter(PromptState::Invisible),   hide::<ui::PromptContainer>)
            .add_systems(OnEnter(CompletionState::Visible), show::<ui::CompletionContainer>)
            .add_systems(OnExit(CompletionState::Visible),  hide::<ui::CompletionContainer>)
            ;
    }
}
