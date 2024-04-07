use crate::event::{DispatchEvent, LookUpEvent, RunActEvent, run_acts};
use crate::lookup::AutoComplete;
use crate::prompt::{
    dispatch_events, look_up_events, hide, hide_delayed, hide_prompt_maybe,
    listen_prompt_active, show, CompletionState, PromptState,
};
use asky::bevy::{AskyPlugin, AskyPrompt};
use bevy::{ecs::system::Resource, reflect::Reflect, text::TextStyle};
use bevy_crossbeam_event::CrossbeamEventApp;
use bevy_input_sequence::*;
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
}

#[rustfmt::skip]
impl bevy::app::Plugin for MinibufferPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        use bevy::prelude::*;
        use bevy::ecs::schedule::{OnEnter, OnExit};
        use super::*;
        use act::*;
        use task::*;
        use ui::*;

        if let Some(type_registry) = app.world.get_resource_mut::<AppTypeRegistry>() {
            let mut type_registry = type_registry.write();
            type_registry.register::<PromptState>();
            type_registry.register::<CompletionState>();
            type_registry.register::<Config>();
        }
        app
            .add_plugins(AskyPlugin)
            .add_key_sequence_event_run_if::<RunActEvent, _>(in_state(AskyPrompt::Inactive))
            .init_state::<PromptState>()
            .init_state::<CompletionState>()
            .insert_resource(self.config.clone())
            .insert_resource(MinibufferStyle {
                text_style: Some(self.config.text_style.clone()),
                ..default()
            })
            .add_crossbeam_event::<DispatchEvent>()
            .add_event::<LookUpEvent>()
            .add_systems(Update, asky::bevy::asky_system::<AutoComplete<asky::Text>>)
            .add_systems(PostUpdate, (dispatch_events, look_up_events).chain())
            .add_systems(Startup,   spawn_layout)
            .add_systems(PreUpdate, run_acts)
            .add_systems(Update,    hide_prompt_maybe)
            .add_systems(Update,    detect_additions::<RunActEvent>)
            .add_systems(Update,    poll_event_tasks::<RunActEvent>)
            .add_systems(PostUpdate, task::poll_tasks_err::<(), Error>)
            // .add_systems(Update,    mouse_scroll)
            .add_systems(Update, listen_prompt_active)
            .add_systems(OnEnter(PromptState::Finished), hide_delayed::<PromptContainer>)
            .add_systems(OnEnter(PromptState::Visible), show::<PromptContainer>)
            .add_systems( OnEnter(PromptState::Invisible), hide::<PromptContainer>)
            .add_systems(OnEnter(CompletionState::Visible), show::<CompletionContainer>)
            .add_systems( OnExit(CompletionState::Visible), hide::<CompletionContainer>)
            ;
    }
}
