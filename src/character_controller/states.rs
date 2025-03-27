use avian3d::math::Vector2;
use bevy::math::{EulerRot, Quat, Vec3};
use bevy::prelude::{Commands, Entity, EventReader, ParamSet, Query, Res, Time, Transform, With, Without};
use crate::camera::ThirdPersonCamera;
use crate::character_controller::MovementAction;
use crate::player::Player;
// Enhanced system to update player states including roll and block
pub fn update_player_states(
    time: Res<Time>,
    mut movement_events: EventReader<MovementAction>,
    mut player_query: Query<(&mut Player, &Transform)>,
    camera_query: Query<&Transform, (With<ThirdPersonCamera>, Without<Player>)>,
) {
    let (Ok((mut player, _player_transform)), Ok(camera_transform)) =
        (player_query.get_single_mut(), camera_query.get_single()) else {
        return;
    };

    let delta = time.delta_secs();

    // Default to not moving/sprinting unless we see a Move event
    player.is_moving = false;
    let mut sprint_requested = false;
    let mut roll_requested = false;
    let mut roll_direction = Vector2::ZERO;
    let mut block_start_requested = false;
    let mut block_end_requested = false;

    // Process all movement events for this frame
    for event in movement_events.read() {
        match event {
            MovementAction::Move(direction, sprinting) => {
                if direction.length_squared() > 0.0 {
                    player.is_moving = true;
                    // Only consider sprinting if movement keys are pressed
                    if *sprinting {
                        sprint_requested = true;
                    }
                }
            },
            MovementAction::Roll(direction) => {
                roll_requested = true;
                roll_direction = *direction;
            },
            MovementAction::StartBlock => {
                block_start_requested = true;
            },
            MovementAction::EndBlock => {
                block_end_requested = true;
            },
            _ => {}
        }
    }

    // Handle roll state and timer
    if player.is_rolling {
        player.roll_timer -= delta;
        if player.roll_timer <= 0.0 {
            // Roll finished
            player.is_rolling = false;
            player.roll_timer = 0.0;
            // Start cooldown
            player.roll_cooldown_timer = player.roll_cooldown;
            player.can_roll = false;
        }
    } else if !player.can_roll {
        // Handle roll cooldown
        player.roll_cooldown_timer -= delta;
        if player.roll_cooldown_timer <= 0.0 {
            player.can_roll = true;
            player.roll_cooldown_timer = 0.0;
        }
    }

    // Process new roll request if player can roll and has stamina
    if roll_requested && player.can_roll && !player.is_rolling && !player.exhausted && player.stamina >= player.roll_stamina_cost {
        // Start rolling
        player.is_rolling = true;
        player.roll_timer = player.roll_duration;

        // Convert input direction to world space using camera orientation
        let camera_yaw = Quat::from_rotation_y(camera_transform.rotation.to_euler(EulerRot::YXZ).0);
        let local_direction = Vec3::new(roll_direction.x, 0.0, -roll_direction.y);
        player.roll_direction = camera_yaw * local_direction;

        // Consume stamina
        player.stamina -= player.roll_stamina_cost;
        if player.stamina < 0.0 {
            player.stamina = 0.0;
        }

        // End blocking if player was blocking
        player.is_blocking = false;
    }

    // Handle blocking state changes
    if block_start_requested && !player.is_rolling && !player.exhausted {
        player.is_blocking = true;
    }

    if block_end_requested || player.is_rolling {
        player.is_blocking = false;
    }

    // Apply stamina cost for blocking
    if player.is_blocking {
        player.stamina -= player.block_stamina_cost_per_sec * delta;

        // Stop blocking if stamina depletes
        if player.stamina <= 0.0 {
            player.stamina = 0.0;
            player.exhausted = true;
            player.exhaustion_timer = 1.0;
            player.is_blocking = false;
        }
    }

    // Handle sprinting state and stamina
    if !player.is_rolling && !player.is_blocking && sprint_requested && !player.exhausted && player.stamina > 0.0 {
        // Player wants to sprint and has stamina
        player.is_sprinting = true;
        player.current_speed = player.run_speed;

        // Reduce stamina while sprinting
        player.stamina -= player.stamina_use_rate * delta;
        if player.stamina <= 0.0 {
            player.stamina = 0.0;
            player.exhausted = true;
            player.exhaustion_timer = 1.0; // 1 second cooldown before regen
        }
    } else if !player.is_rolling {
        // Set speed based on blocking state
        player.is_sprinting = false;
        if player.is_blocking && player.can_move_while_blocking {
            player.current_speed = player.walk_speed * player.block_movement_penalty;
        } else if !player.is_blocking {
            player.current_speed = player.walk_speed;
        }

        // Handle stamina regeneration when not using stamina abilities
        if player.exhausted {
            // Count down exhaust timer when exhausted
            player.exhaustion_timer -= delta;
            if player.exhaustion_timer <= 0.0 {
                player.exhausted = false;
            }
        } else if !sprint_requested && !player.is_rolling && !player.is_blocking && player.stamina < player.max_stamina {
            // Regenerate stamina when not using stamina
            player.stamina += player.stamina_regen_rate * delta;
            player.stamina = player.stamina.min(player.max_stamina);
        }
    }

    // Handle coyote time for jump improvements
    if player.coyote_timer > 0.0 {
        player.coyote_timer -= delta;
    }
}