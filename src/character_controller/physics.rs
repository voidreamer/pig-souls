use avian3d::math::{AdjustPrecision, Vector};
use avian3d::position::Rotation;
use avian3d::prelude::{GravityScale, LinearVelocity, ShapeHits};
use bevy::color::Color;
use bevy::math::{EulerRot, Quat, Vec3};
use bevy::prelude::{Commands, Entity, EventReader, Gizmos, ParamSet, Query, Res, Time, Transform, With};
use crate::camera::ThirdPersonCamera;
use crate::character_controller::components::*;
use crate::character_controller::MovementAction;
use crate::player::Player;

/// Custom gravity system for improved jump feel
pub fn enhanced_gravity(
    mut player_query: Query<(&Player, &mut GravityScale)>,
    mut linear_velocity_query: Query<&mut LinearVelocity, With<Player>>,
) {
    if let (Ok((player, mut gravity_scale)), Ok(linear_velocity)) =
        (player_query.single_mut(), linear_velocity_query.single_mut()) {

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
        Option<&GroundNormal>,
        Option<&Grounded>,
    ), With<CharacterController>>,
) {
    let delta_time = time.delta_secs_f64().adjust_precision();

    // Get camera transform first
    let camera_transform = {
        let camera_query = player_camera_set.p0();
        if let Ok(transform) = camera_query.single() {
            *transform
        } else {
            return;
        }
    };

    // Extract the camera's yaw rotation
    let camera_yaw = Quat::from_rotation_y(camera_transform.rotation.to_euler(EulerRot::YXZ).0);

    // Now get the player query
    let mut player_query = player_camera_set.p1();
    let (mut player, mut player_transform) = player_query.single_mut().expect("No player found");

    // Handle rolling motion if player is rolling
    if player.is_rolling {
        for (_, _, mut linear_velocity, _, _, _) in &mut controllers {
            // Apply roll velocity
            let roll_velocity = player.roll_direction * player.roll_speed * delta_time;
            linear_velocity.x = roll_velocity.x;
            linear_velocity.z = roll_velocity.z;
        }
        return;
    }

    // If blocking and can't move while blocking, zero velocity and return
    if player.is_blocking && !player.can_move_while_blocking {
        for (_, _, mut linear_velocity, _, _, _) in &mut controllers {
            linear_velocity.x = 0.0;
            linear_velocity.z = 0.0;
        }
        return;
    }

    // Normal movement processing
    for event in movement_event_reader.read() {
        for (_, jump_impulse, mut linear_velocity, _, ground_normal, grounded) in &mut controllers {
            match event {
                MovementAction::Move(movement, _) => {
                    if movement.length_squared() > 0.0 {
                        // Convert input direction
                        let movement_local = Vec3::new(movement.x, 0.0, -movement.y);
                        let movement_world = camera_yaw * movement_local;

                        // Store normalized direction
                        player.movement_direction = movement_world.normalize();

                        // Apply slope adjustments if on ground
                        if grounded.is_some() && ground_normal.is_some() {
                            let normal = ground_normal.unwrap().normal();

                            // Only adjust for non-vertical slopes
                            if (normal - Vector::Y).length_squared() > 0.001 {
                                // Calculate slope dot product
                                let slope_dot = movement_world.normalize().dot(Vec3::new(normal.x, 0.0, normal.z).normalize());

                                // Calculate slope factor
                                let slope_factor = if slope_dot < 0.0 {
                                    // Uphill - slowed down
                                    1.0 - slope_dot.abs() * 0.4
                                } else {
                                    // Downhill - speed up
                                    1.0 + slope_dot * 0.3
                                };

                                // Apply slope-adjusted velocity
                                linear_velocity.x = movement_world.x * player.current_speed * delta_time * slope_factor;
                                linear_velocity.z = movement_world.z * player.current_speed * delta_time * slope_factor;
                            } else {
                                // Normal movement on flat ground
                                linear_velocity.x = movement_world.x * player.current_speed * delta_time;
                                linear_velocity.z = movement_world.z * player.current_speed * delta_time;
                            }
                        } else {
                            // Regular movement in air
                            linear_velocity.x = movement_world.x * player.current_speed * delta_time;
                            linear_velocity.z = movement_world.z * player.current_speed * delta_time;
                        }

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
                    // Allow jump if grounded OR within coyote time
                    let can_jump = grounded.is_some() || player.coyote_timer > 0.0;

                    if can_jump {
                        // Apply jump force - simplified for reliability
                        linear_velocity.y = jump_impulse.0;

                        // If on ground and we have a normal, add some directional impulse
                        if grounded.is_some() && ground_normal.is_some() {
                            let normal = ground_normal.unwrap().normal();

                            // Add a small horizontal component based on ground normal
                            linear_velocity.x += normal.x * jump_impulse.0 * 0.3;
                            linear_velocity.z += normal.z * jump_impulse.0 * 0.3;
                        }

                        // Reset coyote timer
                        player.coyote_timer = 0.0;
                    }
                }
                _ => {}
            }
        }
    }

    // Update coyote timer based on grounded state
    let is_player_grounded = controllers.iter().any(|(_, _, _, _, _, grounded)| grounded.is_some());

    if !is_player_grounded && player.coyote_timer <= 0.0 {
        // Just left the ground, start coyote timer
        player.coyote_timer = player.coyote_time;
    } else if !is_player_grounded {
        // In air, count down coyote timer
        player.coyote_timer -= time.delta_secs();
        player.coyote_timer = player.coyote_timer.max(0.0);
    }
}

pub fn update_grounded(
    mut commands: Commands,
    mut query: Query<
        (Entity, &ShapeHits, &Transform, &Rotation, Option<&MaxSlopeAngle>, Option<&mut GroundNormal>),
        With<CharacterController>,
    >,
) {
    for (entity, hits, transform, rotation, max_slope_angle, ground_normal_opt) in &mut query {
        let mut is_grounded = false;
        let mut best_normal = Vector::Y; // Default to up
        let mut best_angle = std::f32::consts::PI; // Start with worst case

        // Get maximum allowed slope angle (default to 45 degrees if not specified)
        let max_allowed_angle = max_slope_angle.map_or(45.0_f32.to_radians(), |angle| angle.0);

        // Get current movement direction (use transform's forward direction)
        let movement_direction = transform.forward();

        // Check if we have any ground contacts
        if !hits.is_empty() {
            // Find the best ground normal
            for hit in hits.iter() {
                // Convert the hit normal to world space
                let normal = rotation * -hit.normal2;

                // Calculate angle with vertical
                let angle = normal.angle_between(Vector::Y).abs();

                // For very steep slopes, we still want visual rotation even if not "grounded"
                // This ensures the character visually aligns with the slope
                if angle <= 1.2 * max_allowed_angle { // 20% more lenient for visual alignment
                    // Get projected movement direction (flat)
                    let flat_movement = Vec3::new(movement_direction.x, 0.0, movement_direction.z).normalize();

                    // Calculate uphill/downhill factor
                    let slope_direction = Vec3::new(normal.x, 0.0, normal.z).normalize();
                    let uphill_factor = flat_movement.dot(slope_direction);

                    // Be more lenient when evaluating uphill movement
                    let effective_angle = if uphill_factor < 0.0 {
                        // Going uphill - reduce effective angle based on steepness
                        angle * (0.9 - 0.1 * (angle / max_allowed_angle).min(1.0))
                    } else {
                        angle
                    };

                    // If this is better than our current best
                    if effective_angle < best_angle {
                        best_normal = normal;
                        best_angle = effective_angle;

                        // Only set as "grounded" if within actual max slope angle
                        if angle <= max_allowed_angle {
                            is_grounded = true;
                        }
                    }
                }
            }
        }

        // If we have a ground normal component, update it - even for steep slopes!
        if let Some(mut ground_normal) = ground_normal_opt {
            if !hits.is_empty() { // If we have any hits at all
                // Gradually approach the best normal for smoother transitions
                let current = ground_normal.normal();
                let target = best_normal;

                // Use a faster transition when the change is significant
                let angle_diff = current.angle_between(target);
                let blend_rate = if angle_diff > 0.2 {
                    // Faster adjustment for big changes
                    0.3
                } else {
                    // Slower adjustment for refinement
                    0.15
                };

                let blended = current.lerp(target, blend_rate);
                ground_normal.set_normal(blended);
            } else {
                // If no hits at all, gradually return to vertical
                let current = ground_normal.normal();
                let target = Vector::Y;
                let blended = current.lerp(target, 0.1);
                ground_normal.set_normal(blended);
            }
        }

        // Update grounded state - only for valid slope angles
        if is_grounded {
            commands.entity(entity).insert(Grounded);
        } else {
            commands.entity(entity).remove::<Grounded>();
        }
    }
}
pub fn update_character_visual_tilt(
    time: Res<Time>,
    mut query: Query<(&GroundNormal, &mut Transform, Option<&Player>)>,
) {
    for (ground_normal, mut transform, player) in &mut query {
        let normal = ground_normal.normal();

        // Skip if nearly vertical
        if (normal - Vector::Y).length_squared() < 0.001 {
            continue;
        }

        // Get current forward direction - important to preserve the character's heading
        let (yaw, _, _) = transform.rotation.to_euler(EulerRot::YXZ);
        let heading_rotation = Quat::from_rotation_y(yaw);
        let forward = heading_rotation * Vec3::Z;

        // For character rotation, we want to pitch along the forward/backward axis
        // This ensures the character pitches up/down as expected on slopes

        // 1. Project the normal onto the forward-up plane
        let forward_flat = Vec3::new(forward.x, 0.0, forward.z).normalize();
        let right = Vec3::Y.cross(forward_flat).normalize();

        // 2. Calculate pitch angle based on slope
        let pitch_component = Vec3::new(
            normal.dot(right), // This should be near zero for proper pitching
            normal.y,           // Up component
            normal.dot(-forward_flat) // Forward component (negative since model faces -Z)
        ).normalize();

        // 3. Calculate the necessary rotation to go from up to our slope normal
        let up_vector = Vec3::Y;
        let pitch_angle = up_vector.angle_between(pitch_component);

        // 4. This is our rotation axis (perpendicular to both up and forward)
        let pitch_axis = right; // Right vector gives us rotation around the model's side-to-side axis

        // Create quaternion for just the slope tilt
        let slope_rotation = if pitch_angle > 0.01 {
            Quat::from_axis_angle(pitch_axis, pitch_angle)
        } else {
            Quat::IDENTITY
        };

        // Combine with heading to maintain forward direction
        let target_rotation = heading_rotation * slope_rotation;

        // Use a faster adjustment for player entity
        let lerp_speed = if player.is_some() { 8.0 } else { 5.0 };

        // Smoothly interpolate to target rotation
        transform.rotation = transform.rotation.slerp(
            target_rotation,
            time.delta_secs() * lerp_speed
        );
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


pub fn debug_visualize_ground_normals(
    mut gizmos: Gizmos,
    query: Query<(&GroundNormal, &Transform, Option<&Grounded>, Option<&MaxSlopeAngle>), With<Player>>,
) {
    for (ground_normal, transform, grounded, max_slope_angle) in &query {
        let origin = transform.translation + Vec3::new(0.0, 0.5, 0.0); // Move up slightly for visibility
        let normal = ground_normal.normal();

        // Get max slope angle (default to 45 degrees if not specified)
        let max_allowed_angle = max_slope_angle.map_or(45.0_f32.to_radians(), |angle| angle.0);

        // Calculate slope angle from vertical
        let slope_angle = normal.angle_between(Vector::Y).abs();

        // Determine color based on grounded state and slope steepness
        let color = if grounded.is_some() {
            Color::srgb(0.0, 1.0, 0.0)
        } else if slope_angle <= 1.2 * max_allowed_angle {
            Color::srgb(1.0, 1.0, 0.0) // Too steep for physics, but we allow visual tilt
        } else {
            Color::srgb(1.0, 0.0, 0.0) // Far too steep - not used for anything
        };

        // Draw the ground normal as a line
        gizmos.line(
            origin,
            origin + Vec3::new(normal.x, normal.y, normal.z) * 3.0,
            color
        );

        // Draw the up vector for comparison
        gizmos.line(
            origin,
            origin + Vec3::Y * 3.0,
            Color::srgb(0.0, 0.0, 1.0)
        );

        // Draw a small sphere at the origin point for clarity
        gizmos.sphere(origin, 0.1, color);
    }
}