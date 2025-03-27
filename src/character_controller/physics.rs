use avian3d::math::{AdjustPrecision, Vector};
use avian3d::position::Rotation;
use avian3d::prelude::{GravityScale, LinearVelocity, ShapeHits};
use bevy::math::{EulerRot, Quat, Vec3};
use bevy::prelude::{Commands, Entity, EventReader, ParamSet, Query, Res, Time, Transform, With};
use crate::camera::ThirdPersonCamera;
use crate::character_controller::components::{CharacterController, Grounded, JumpImpulse, MaxSlopeAngle, MovementAcceleration, MovementDampingFactor};
use crate::character_controller::MovementAction;
use crate::player::Player;

/// Custom gravity system for improved jump feel
pub fn enhanced_gravity(
    mut player_query: Query<(&Player, &mut GravityScale)>,
    mut linear_velocity_query: Query<&mut LinearVelocity, With<Player>>,
) {
    if let (Ok((player, mut gravity_scale)), Ok(linear_velocity)) =
        (player_query.get_single_mut(), linear_velocity_query.get_single_mut()) {

        // If we're falling, increase gravity
        if linear_velocity.y < 0.0 {
            // Apply fall multiplier for faster descent
            gravity_scale.0 = 2.0 * player.fall_multiplier;
        }
        // If we're rising but jump button was released, apply low jump multiplier
        else if linear_velocity.y > 0.0 {
            gravity_scale.0 = 2.0;
        }
        else {
            // Default gravity scale
            gravity_scale.0 = 2.0;
        }
    }
}

/// Handles movement including rolling state
pub fn movement(
    time: Res<Time>,
    mut movement_event_reader: EventReader<MovementAction>,
    mut player_camera_set: ParamSet<(
        Query<&Transform, With<ThirdPersonCamera>>,
        Query<(&mut Player, &mut Transform)>,
    )>,
    mut controllers: Query<(
        &MovementAcceleration,
        &JumpImpulse,
        &mut LinearVelocity,
        Entity,
    )>,
) {
    let delta_time = time.delta_secs_f64().adjust_precision();

    // Get camera transform first and store it outside the query
    let camera_transform = {
        let camera_query = player_camera_set.p0();
        if let Ok(transform) = camera_query.get_single() {
            *transform // Clone the transform
        } else {
            return; // No camera found, can't determine direction
        }
    }; // Camera query borrow ends here

    // Extract the camera's yaw rotation (around Y axis)
    let camera_yaw = Quat::from_rotation_y(camera_transform.rotation.to_euler(EulerRot::YXZ).0);

    // Now get the player query after the camera query borrow is dropped
    let mut player_query = player_camera_set.p1();
    let (mut player, mut player_transform) = player_query.single_mut();

    // Handle rolling motion if player is rolling
    if player.is_rolling {
        for (_, _, mut linear_velocity, _) in &mut controllers {
            // Apply roll velocity
            let roll_velocity = player.roll_direction * player.roll_speed * delta_time;
            linear_velocity.x = roll_velocity.x as f32;
            linear_velocity.z = roll_velocity.z as f32;
        }

        // Skip normal movement processing if rolling
        return;
    }

    // If blocking and can't move while blocking, zero velocity and return
    if player.is_blocking && !player.can_move_while_blocking {
        for (_, _, mut linear_velocity, _) in &mut controllers {
            linear_velocity.x = 0.0;
            linear_velocity.z = 0.0;
        }
        return;
    }

    // Normal movement processing
    for event in movement_event_reader.read() {
        for (_, jump_impulse, mut linear_velocity, _entity) in &mut controllers {
            match event {
                MovementAction::Move(movement, _) => {
                    if movement.length_squared() > 0.0 {
                        // Convert the input direction to be relative to camera orientation
                        let movement_local = Vec3::new(movement.x, 0.0, -movement.y);

                        // Then rotate it by the camera's yaw
                        let movement_world = camera_yaw * movement_local;

                        // Store normalized direction for rotation
                        player.movement_direction = movement_world.normalize();

                        // Apply movement velocity
                        linear_velocity.x = movement_world.x * player.current_speed * delta_time;
                        linear_velocity.z = movement_world.z * player.current_speed * delta_time;

                        // Rotate player to face movement direction
                        let target_rotation = Quat::from_rotation_y(
                            f32::atan2(movement_world.x, movement_world.z)
                        );

                        // Smoothly interpolate rotation
                        player_transform.rotation = player_transform.rotation.slerp(
                            target_rotation,
                            10.0 * time.delta_secs()
                        );
                    }
                }
                MovementAction::Jump => {
                    // Allow jumping if grounded or within coyote time
                    if player.coyote_timer > 0.0 {
                        linear_velocity.y = jump_impulse.0;
                        player.coyote_timer = 0.0; // Reset coyote timer after jump
                    }
                }
                _ => {}
            }

            // Start coyote timer when leaving ground
            if player.coyote_timer <= 0.0 {
                player.coyote_timer = player.coyote_time;
            }
        }
    }
}

pub fn update_grounded(
    mut commands: Commands,
    mut query: Query<
        (Entity, &ShapeHits, &Rotation, Option<&MaxSlopeAngle>),
        With<CharacterController>,
    >,
) {
    for (entity, hits, rotation, max_slope_angle) in &mut query {
        // The character is grounded if the shape caster has a hit with a normal
        // that isn't too steep.
        let is_grounded = hits.iter().any(|hit| {
            if let Some(angle) = max_slope_angle {
                (rotation * -hit.normal2).angle_between(Vector::Y).abs() <= angle.0
            } else {
                true
            }
        });

        if is_grounded {
            commands.entity(entity).insert(Grounded);
        } else {
            commands.entity(entity).remove::<Grounded>();
        }
    }
}

/// Slows down movement in the XZ plane when no input is given
pub fn apply_movement_damping(
    mut event_reader: EventReader<MovementAction>,
    mut query: Query<(&MovementDampingFactor, &mut LinearVelocity)>
) {
    // Check if any movement occurred this frame
    let mut moving = false;
    for event in event_reader.read() {
        if let MovementAction::Move(dir, _) = event {
            if dir.length_squared() > 0.0 {
                moving = true;
                break;
            }
        }
    }

    // Only apply damping if not actively moving
    if !moving {
        for (damping_factor, mut linear_velocity) in &mut query {
            // We could use `LinearDamping`, but we don't want to dampen movement along the Y axis
            linear_velocity.x *= damping_factor.0;
            linear_velocity.z *= damping_factor.0;
        }
    }
}