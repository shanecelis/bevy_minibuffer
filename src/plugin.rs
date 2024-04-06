use std::borrow::Cow;
use bevy::{ecs::system::Resource, reflect::Reflect, text::TextStyle};
use asky::bevy::{AskyPlugin, AskyPrompt};
use bevy_crossbeam_event::CrossbeamEventApp;
use bevy_input_sequence::*;
use crate::lookup::{AutoComplete};
use crate::event::{DispatchEvent, LookUpEvent, StartActEvent};
use crate::prompt::{PromptState, CompletionState,
                    handle_dispatch_event, handle_look_up_event, hide, hide_prompt_maybe,
                    listen_prompt_active, hide_delayed, show};

#[derive(Debug, Default, Clone)]
pub struct MinibufferPlugin {
    pub config: ConsoleConfig,
}

#[derive(Debug, Resource, Clone, Default, Reflect)]
pub struct ConsoleConfig {
    pub auto_hide: bool,
    pub hide_delay: Option<u64>, // milliseconds
    pub text_style: TextStyle,
}

// impl Default for ConsoleConfig {
//     fn default() -> Self {
//         Self {
//             hide_delay: Some(2000), /* milliseconds */
//         }
//     }
// }

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Message(Cow<'static, str>),
    #[error("asky {0}")]
    Asky(#[from] asky::Error),
}

#[rustfmt::skip]
impl bevy::app::Plugin for MinibufferPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        use bevy::prelude::*;
        use bevy::ecs::schedule::{OnEnter, OnExit};
        use super::*;
        use commands::*;
        use tasks::*;
        use ui::*;

        if let Some(type_registry) = app.world.get_resource_mut::<AppTypeRegistry>() {
            let mut type_registry = type_registry.write();
            type_registry.register::<PromptState>();
            type_registry.register::<CompletionState>();
            type_registry.register::<ConsoleConfig>();
        }
        app
            .add_plugins(AskyPlugin)
            .add_key_sequence_event_run_if::<StartActEvent, _>(in_state(AskyPrompt::Inactive))
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
            .add_systems(PostUpdate, (handle_dispatch_event, handle_look_up_event).chain())
            .add_systems(Startup,   spawn_layout)
            .add_systems(PreUpdate, run_command_listener)
            .add_systems(Update,    hide_prompt_maybe)
            .add_systems(Update,    detect_additions::<StartActEvent>)
            .add_systems(Update,    poll_event_tasks::<StartActEvent>)
            .add_systems(PostUpdate, tasks::poll_tasks_err::<(), Error>)
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
