// #![feature(return_position_impl_trait_in_trait)]
#![allow(incomplete_features)]
#![feature(async_fn_in_trait)]
pub mod commands;
pub mod prompt;
pub mod tasks;
pub mod ui;

pub struct NanoPromptPlugin;
impl bevy::app::Plugin for NanoPromptPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        use tasks::*;
        use prompt::*;
        use commands::*;
        use ui::*;
        use bevy::app::*;
        use bevy::ecs::schedule::{OnEnter, OnExit};
        app.add_event::<RunCommandEvent>()
            .add_state::<PromptState>()
            .add_state::<CompletionState>()
            .init_resource::<PromptProvider>()
            .init_resource::<CommandConfig>()
            .add_systems(Startup,   spawn_layout)
            .add_systems(PreUpdate, run_commands)
            .add_systems(Update,    hide_prompt_maybe)
            .add_systems(Update,    prompt_input)
            .add_systems(Update,    poll_tasks)
            .add_systems(Update,    poll_event_tasks::<RunCommandEvent>)
            .add_systems(Update,    mouse_scroll)
            .add_systems(Update,    hotkey_input)
            .add_systems(OnEnter(PromptState::Visible), show::<PromptContainer>)
            .add_systems( OnExit(PromptState::Visible), hide_delayed::<PromptContainer>)
            .add_systems(OnEnter(CompletionState::Visible), show::<CompletionContainer>)
            .add_systems( OnExit(CompletionState::Visible), hide::<CompletionContainer>)
            ;
    }
}
