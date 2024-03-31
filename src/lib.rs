// #![feature(return_position_impl_trait_in_trait)]
#![allow(incomplete_features)]
pub mod commands;
pub mod hotkey;
pub mod prompt;
pub mod tasks;
pub mod ui;
pub mod style;
use style::MinibufferStyle;
use bevy_input_sequence::*;
use asky::bevy::{AskyPlugin, AskyPrompt};
use bevy::ecs::schedule::common_conditions::in_state;
pub use prompt::Minibuffer;
use bevy_crossbeam_event::CrossbeamEventApp;
use trie_rs::map::Trie;

pub struct NanoPromptPlugin;
#[rustfmt::skip]
impl bevy::app::Plugin for NanoPromptPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        use bevy::app::*;
        use bevy::ecs::schedule::{OnEnter, OnExit};
        use commands::*;
        use prompt::*;
        use tasks::*;
        use ui::*;
        app
            .add_plugins(AskyPlugin)
            .add_key_sequence_event_run_if::<StartActEvent, _>(in_state(AskyPrompt::Inactive))
            .init_state::<PromptState>()
            .init_state::<CompletionState>()
            .init_resource::<ConsoleConfig>()
            .add_crossbeam_event::<LookUpEvent>()
            .add_systems(Update, asky::bevy::asky_system::<AutoComplete<asky::Text>>)
            .add_systems(PostUpdate, handle_look_up_event)
            .add_systems(Startup,   spawn_layout)
            .add_systems(PreUpdate, run_command_listener)
            .add_systems(Update,    hide_prompt_maybe)
            .add_systems(Update,    detect_additions::<StartActEvent>)
            .add_systems(Update,    poll_event_tasks::<StartActEvent>)
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
