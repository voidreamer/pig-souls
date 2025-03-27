use avian3d::math::{Scalar, Vector2};
use bevy::input::ButtonInput;
use bevy::prelude::{EventWriter, Gamepad, GamepadAxis, GamepadButton, KeyCode, MouseButton, Query, Res};
use crate::character_controller::MovementAction;
use crate::player::Player;

/// Sends [`MovementAction`] events based on keyboard input.
pub fn keyboard_input(
    mut movement_event_writer: EventWriter<MovementAction>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mouse_input: Res<ButtonInput<MouseButton>>,
    player_query: Query<&Player>,
) {
    let Ok(player) = player_query.get_single() else { return };

    // Basic movement
    let up = keyboard_input.any_pressed([KeyCode::KeyW, KeyCode::ArrowUp]);
    let down = keyboard_input.any_pressed([KeyCode::KeyS, KeyCode::ArrowDown]);
    let left = keyboard_input.any_pressed([KeyCode::KeyA, KeyCode::ArrowLeft]);
    let right = keyboard_input.any_pressed([KeyCode::KeyD, KeyCode::ArrowRight]);

    // Check if sprinting (any shift key)
    let sprinting = keyboard_input.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);

    // Calculate movement direction
    let horizontal = right as i8 - left as i8;
    let vertical = up as i8 - down as i8;
    let direction = Vector2::new(horizontal as Scalar, vertical as Scalar).clamp_length_max(1.0);

    // Send movement event if there's input and not rolling
    if direction != Vector2::ZERO && !player.is_rolling {
        movement_event_writer.send(MovementAction::Move(direction, sprinting));
    }

    // Handle jump
    if keyboard_input.just_pressed(KeyCode::Space) && !player.is_rolling {
        movement_event_writer.send(MovementAction::Jump);
    }

    // Handle roll
    if keyboard_input.just_pressed(KeyCode::ControlLeft) && player.can_roll && !player.is_rolling && !player.exhausted {
        // Use the current movement direction for rolling, or forward if not moving
        let roll_direction = if direction != Vector2::ZERO {
            direction
        } else {
            Vector2::new(0.0, 1.0) // Default to forward
        };

        movement_event_writer.send(MovementAction::Roll(roll_direction));
    }

    // Handle blocking (right mouse button)
    if mouse_input.just_pressed(MouseButton::Right) && !player.is_rolling {
        movement_event_writer.send(MovementAction::StartBlock);
    }
    if mouse_input.just_released(MouseButton::Right) && player.is_blocking {
        movement_event_writer.send(MovementAction::EndBlock);
    }
}

/// Sends [`MovementAction`] events based on gamepad input.
pub fn gamepad_input(
    mut movement_event_writer: EventWriter<MovementAction>,
    gamepads: Query<&Gamepad>,
    player_query: Query<&Player>,
) {
    let Ok(player) = player_query.get_single() else { return };

    for gamepad in gamepads.iter() {
        // Movement with left stick
        if let (Some(x), Some(y)) = (
            gamepad.get(GamepadAxis::LeftStickX),
            gamepad.get(GamepadAxis::LeftStickY),
        ) {
            // Use Right Trigger or Right Shoulder for sprinting in gamepad
            let sprint = gamepad.pressed(GamepadButton::RightTrigger2) ||
                gamepad.pressed(GamepadButton::RightTrigger2);

            let direction = Vector2::new(x as Scalar, y as Scalar).clamp_length_max(1.0);

            // Only send movement if not rolling
            if direction.length_squared() > 0.01 && !player.is_rolling {
                movement_event_writer.send(MovementAction::Move(direction, sprint));
            }
        }

        // Jump (A/Cross button)
        if gamepad.just_pressed(GamepadButton::South) && !player.is_rolling {
            movement_event_writer.send(MovementAction::Jump);
        }

        // Roll (B/Circle button)
        if gamepad.just_pressed(GamepadButton::East) && player.can_roll && !player.is_rolling && !player.exhausted {
            // Get current direction from left stick
            let x = gamepad.get(GamepadAxis::LeftStickX).unwrap_or(0.0);
            let y = gamepad.get(GamepadAxis::LeftStickY).unwrap_or(0.0);
            let direction = Vector2::new(x as Scalar, y as Scalar);

            // Use current direction, or forward if stick is neutral
            let roll_direction = if direction.length_squared() > 0.01 {
                direction.clamp_length_max(1.0)
            } else {
                Vector2::new(0.0, 1.0) // Default to forward
            };

            movement_event_writer.send(MovementAction::Roll(roll_direction));
        }

        // Block with R2/Right Trigger
        if gamepad.just_pressed(GamepadButton::RightTrigger) && !player.is_rolling {
            movement_event_writer.send(MovementAction::StartBlock);
        }
        if gamepad.just_released(GamepadButton::RightTrigger) && player.is_blocking {
            movement_event_writer.send(MovementAction::EndBlock);
        }
    }
}