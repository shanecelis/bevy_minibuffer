use crate::{
    universal,
    act,
    event::{run_acts, DispatchEvent, LookUpEvent, RunActEvent, RunInputSequenceEvent},
    lookup::AutoComplete,
    prompt::{
        self, dispatch_events, get_key_chords, hide, hide_delayed, hide_prompt_maybe,
        listen_prompt_active, look_up_events, show, CompletionState, MinibufferState, PromptState,
    },
    ui,
};
use asky::bevy::AskyPlugin;
use bevy::{
    app::{PostUpdate, PreUpdate, Startup, Update},
    ecs::{
        reflect::AppTypeRegistry,
        schedule::{Condition, common_conditions::{in_state, on_event}, OnEnter, OnExit, SystemSet, IntoSystemSetConfigs},
        system::Resource,
    },
    prelude::IntoSystemConfigs,
    reflect::Reflect,
    text::TextStyle,
    utils::default,
};
use bevy_crossbeam_event::CrossbeamEventApp;
use bevy_input_sequence::{InputSequencePlugin};
use std::borrow::Cow;

/// Minibuffer plugin
#[derive(Debug, Default, Clone)]
pub struct MinibufferPlugin {
    /// Configuration
    pub config: Config,
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
    Asky(#[from] asky::Error),
    /// An async error
    #[error("async error {0}")]
    Async(#[from] bevy_defer::AsyncFailure),
}


#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
enum MinibufferSet {
    Input,
    Process,
    Output
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
struct InputSet;


#[rustfmt::skip]
impl bevy::app::Plugin for MinibufferPlugin {
    fn build(&self, app: &mut bevy::app::App) {

        if let Some(type_registry) = app.world.get_resource_mut::<AppTypeRegistry>() {
            let mut type_registry = type_registry.write();
            type_registry.register::<PromptState>();
            type_registry.register::<CompletionState>();
            type_registry.register::<Config>();
        }
        app
            .add_plugins(AskyPlugin)
            .add_plugins(InputSequencePlugin::empty()
            .run_in_set(Update, InputSet))
            .init_state::<MinibufferState>()
            .init_state::<PromptState>()
            .init_state::<CompletionState>()
            .init_resource::<act::ActCache>()
            .insert_resource(self.config.clone())
            .insert_resource(crate::MinibufferStyle {
                text_style: Some(self.config.text_style.clone()),
                ..default()
            })
            .add_crossbeam_event::<DispatchEvent>()
            .add_event::<RunInputSequenceEvent>()
            .add_event::<LookUpEvent>()
            .add_event::<RunActEvent>()
            .add_systems(Startup, ui::spawn_layout)
            .add_systems(Update,
                         (hide_prompt_maybe,
                          act::detect_additions,
                          asky::bevy::asky_system::<AutoComplete<asky::Text>>,
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
                         ((run_acts,
                           prompt::set_minibuffer_state).chain(),
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
