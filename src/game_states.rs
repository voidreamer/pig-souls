use bevy::app::{App, Plugin};
use bevy::prelude::{AppExtStates, States};

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
pub enum AppState {
    #[default]
    Menu,
    InGame,
    // Inventory,
    // Death
}

pub struct GameStatePlugin;

impl Plugin for GameStatePlugin{
    fn build(&self, app: &mut App) {
        app
            .init_state::<AppState>(); // Alternatively we could use .insert_state(AppState::Menu)
    }
}

