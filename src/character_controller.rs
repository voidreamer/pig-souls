use avian3d::{math::*, prelude::*};
use bevy::{ecs::query::Has, prelude::*};
use crate::camera::ThirdPersonCamera;
use crate::player::Player;

/*
mod controller_input;
mod controller_state;
mod controller_physics;
mod controller_components;

 */

// Re-export the components
pub use controller_components::*;
use crate::game_states::AppState;

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
                    controller_input::keyboard_input,
                    controller_input::gamepad_input,

                    // State management
                    controller_state::update_player_states,

                    // Physics systems
                    controller_physics::enhanced_gravity,
                    controller_physics::update_grounded,
                    controller_physics::movement,
                    controller_physics::apply_movement_damping,
                ).run_if(in_state(AppState::InGame))
                    .chain(),
            );
    }
}

// Define the submodules below

mod controller_components {
    use super::*;

    /// A marker component indicating that an entity is using a character controller.
    #[derive(Component)]
    pub struct CharacterController;

    /// A marker component indicating that an entity is on the ground.
    #[derive(Component)]
    #[component(storage = "SparseSet")]
    pub struct Grounded;

    /// The maximum angle a slope can have for a character controller
    /// to be able to climb and jump. If the slope is steeper than this angle,
    /// the character will slide down.
    #[derive(Component)]
    pub struct MaxSlopeAngle(pub(crate) Scalar);

    /// The acceleration used for character movement.
    #[derive(Component)]
    pub struct MovementAcceleration(pub Scalar);

    /// The damping factor used for slowing down movement.
    #[derive(Component)]
    pub struct MovementDampingFactor(pub Scalar);

    /// The strength of a jump.
    #[derive(Component)]
    pub struct JumpImpulse(pub Scalar);


    /// A bundle that contains the components needed for a basic
    /// kinematic character controller.
    #[derive(Bundle)]
    pub struct CharacterControllerBundle {
        character_controller: CharacterController,
        rigid_body: RigidBody,
        collider: Collider,
        ground_caster: ShapeCaster,
        locked_axes: LockedAxes,
        movement: MovementBundle,
    }

    /// A bundle that contains components for character movement.
    #[derive(Bundle)]
    pub struct MovementBundle {
        acceleration: MovementAcceleration,
        damping: MovementDampingFactor,
        jump_impulse: JumpImpulse,
        max_slope_angle: MaxSlopeAngle,
    }

    impl MovementBundle {
        pub const fn new(
            acceleration: Scalar,
            damping: Scalar,
            jump_impulse: Scalar,
            max_slope_angle: Scalar,
        ) -> Self {
            Self {
                acceleration: MovementAcceleration(acceleration),
                damping: MovementDampingFactor(damping),
                jump_impulse: JumpImpulse(jump_impulse),
                max_slope_angle: MaxSlopeAngle(max_slope_angle),
            }
        }
    }

    impl Default for MovementBundle {
        fn default() -> Self {
            Self::new(30.0, 0.9, 7.0, PI * 0.45)
        }
    }

    impl CharacterControllerBundle {
        pub fn new(collider: Collider) -> Self {
            // Create shape caster as a slightly smaller version of collider
            let mut caster_shape = collider.clone();
            caster_shape.set_scale(Vector::ONE * 0.99, 10);

            Self {
                character_controller: CharacterController,
                rigid_body: RigidBody::Dynamic,
                collider,
                ground_caster: ShapeCaster::new(
                    caster_shape,
                    Vector::ZERO,
                    Quaternion::default(),
                    Dir3::NEG_Y,
                )
                    .with_max_distance(0.2),
                locked_axes: LockedAxes::ROTATION_LOCKED,
                movement: MovementBundle::default(),
            }
        }

        pub fn with_movement(
            mut self,
            acceleration: Scalar,
            damping: Scalar,
            jump_impulse: Scalar,
            max_slope_angle: Scalar,
        ) -> Self {
            self.movement = MovementBundle::new(acceleration, damping, jump_impulse, max_slope_angle);
            self
        }
    }
}

mod controller_input {
    use super::*;

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
}

mod controller_state {
    use super::*;

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
}

mod controller_physics {
    use super::*;

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
}