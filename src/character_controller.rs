mod components;
mod input;
mod states;
mod physics;

use avian3d::math::*;
use bevy::prelude::*;
use crate::game_states::AppState;
pub use components::*;

/// An event sent for a movement input action.
#[derive(Event)]
pub enum MovementAction {
    Move(Vector2, bool), // Direction vector and sprint flag
    Jump,
    Roll(Vector2),      // Direction to roll in
    StartBlock,         // Start blocking
    EndBlock,           // Stop blocking
}

pub struct CharacterControllerPlugin;

impl Plugin for CharacterControllerPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<MovementAction>()
            .add_systems(
                FixedUpdate,
                (
                    // Input processing
                    input::keyboard_input,
                    input::gamepad_input,

                    // State management
                    states::update_player_states,

                    physics::enhanced_gravity,
                    physics::update_grounded,
                    physics::movement,
                    physics::apply_movement_damping,
                ).run_if(in_state(AppState::InGame))
                    .chain(),
            )
            // Visual tilt in Update schedule for smoother animation
            .add_systems(
                FixedUpdate,
                (
                    physics::update_character_visual_tilt,
                    physics::debug_visualize_ground_normals,
                ).run_if(in_state(AppState::InGame))
            );
    }
}