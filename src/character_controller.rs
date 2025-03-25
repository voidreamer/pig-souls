use avian3d::{math::*, prelude::*};
use bevy::{ecs::query::Has, prelude::*};
use crate::camera::ThirdPersonCamera;
use crate::player::Player;

pub struct CharacterControllerPlugin;

impl Plugin for CharacterControllerPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<MovementAction>().add_systems(
            Update,
            (
                keyboard_input,
                gamepad_input,
                update_grounded,
                update_player_sprint_state,
                movement,
                apply_movement_damping,
            )
                .chain(),
        );
    }
}

/// An event sent for a movement input action.
#[derive(Event)]
pub enum MovementAction {
    Move(Vector2, bool), // Direction vector and sprint flag
    Jump,
}

/// A marker component indicating that an entity is using a character controller.
#[derive(Component)]
pub struct CharacterController;

/// A marker component indicating that an entity is on the ground.
#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct Grounded;
/// The acceleration used for character movement.
#[derive(Component)]
pub struct MovementAcceleration(Scalar);

/// The damping factor used for slowing down movement.
#[derive(Component)]
pub struct MovementDampingFactor(Scalar);

/// The strength of a jump.
#[derive(Component)]
pub struct JumpImpulse(Scalar);

/// The maximum angle a slope can have for a character controller
/// to be able to climb and jump. If the slope is steeper than this angle,
/// the character will slide down.
#[derive(Component)]
pub struct MaxSlopeAngle(Scalar);

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

/// Sends [`MovementAction`] events based on keyboard input.
fn keyboard_input(
    mut movement_event_writer: EventWriter<MovementAction>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    let up = keyboard_input.any_pressed([KeyCode::KeyW, KeyCode::ArrowUp]);
    let down = keyboard_input.any_pressed([KeyCode::KeyS, KeyCode::ArrowDown]);
    let left = keyboard_input.any_pressed([KeyCode::KeyA, KeyCode::ArrowLeft]);
    let right = keyboard_input.any_pressed([KeyCode::KeyD, KeyCode::ArrowRight]);

    // Check if sprinting (any shift key)
    let sprinting = keyboard_input.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);

    let horizontal = right as i8 - left as i8;
    let vertical = up as i8 - down as i8;
    let direction = Vector2::new(horizontal as Scalar, vertical as Scalar).clamp_length_max(1.0);

    if direction != Vector2::ZERO {
        movement_event_writer.send(MovementAction::Move(direction, sprinting));
    }

    if keyboard_input.just_pressed(KeyCode::Space) {
        movement_event_writer.send(MovementAction::Jump);
    }
}
/// Sends [`MovementAction`] events based on gamepad input.
fn gamepad_input(
    mut movement_event_writer: EventWriter<MovementAction>,
    gamepads: Query<&Gamepad>,
) {
    for gamepad in gamepads.iter() {
        if let (Some(x), Some(y)) = (
            gamepad.get(GamepadAxis::LeftStickX),
            gamepad.get(GamepadAxis::LeftStickY),
        ) {
            // Use Right Trigger or Right Shoulder for sprinting in gamepad
            let sprint = gamepad.pressed(GamepadButton::RightTrigger2) ||
                gamepad.pressed(GamepadButton::RightTrigger2);

            movement_event_writer.send(MovementAction::Move(
                Vector2::new(x as Scalar, y as Scalar).clamp_length_max(1.0),
                sprint,
            ));
        }

        if gamepad.just_pressed(GamepadButton::South) {
            movement_event_writer.send(MovementAction::Jump);
        }
    }
}

/// Updates the [`Grounded`] status for character controllers.
fn update_grounded(
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

/// Responds to [`MovementAction`] events and moves character controllers accordingly.
fn update_player_sprint_state(
    time: Res<Time>,
    mut movement_events: EventReader<MovementAction>,
    mut player_query: Query<&mut Player>,
) {
    let Ok(mut player) = player_query.get_single_mut() else { return };
    let delta = time.delta_secs();

    // Default to not moving/sprinting unless we see a Move event
    player.is_moving = false;
    let mut sprint_requested = false;

    // Check for movement events
    for event in movement_events.read() {
        if let MovementAction::Move(direction, sprinting) = event {
            if direction.length_squared() > 0.0 {
                player.is_moving = true;

                // Only consider sprinting if movement keys are pressed
                if *sprinting {
                    sprint_requested = true;
                }
            }
        }
    }

    // Handle sprinting state and stamina
    if sprint_requested && !player.exhausted && player.stamina > 0.0 {
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
    } else {
        // Not sprinting (either by choice or exhaustion)
        player.is_sprinting = false;
        player.current_speed = player.walk_speed;

        // Handle stamina regeneration when not sprinting
        if player.exhausted {
            // Count down exhaust timer when exhausted
            player.exhaustion_timer -= delta;
            if player.exhaustion_timer <= 0.0 {
                player.exhausted = false;
            }
        } else if !sprint_requested && player.stamina < player.max_stamina {
            // Regenerate stamina when not sprinting and not exhausted
            player.stamina += player.stamina_regen_rate * delta;
            player.stamina = player.stamina.min(player.max_stamina);
        }
    }
}

// Replace the movement function in character_controller.rs
fn movement(
    time: Res<Time>,
    mut movement_event_reader: EventReader<MovementAction>,
    camera_query: Query<&Transform, With<ThirdPersonCamera>>,
    player_query: Query<&Player>,
    mut controllers: Query<(
        &MovementAcceleration,
        &JumpImpulse,
        &mut LinearVelocity,
        Has<Grounded>,
    )>,
) {
    // Get player speed
    let player_speed = if let Ok(player) = player_query.get_single() {
        player.current_speed
    } else {
        30.0 // Default speed if player not found
    };

    // Precision is adjusted so that the example works with
    // both the `f32` and `f64` features.
    let delta_time = time.delta_secs_f64().adjust_precision();

    // Get camera transform - we'll use this for direction
    let camera_transform = if let Ok(transform) = camera_query.get_single() {
        transform
    } else {
        return; // No camera found, can't determine direction
    };

    // Extract the camera's yaw rotation (around Y axis)
    let camera_yaw = Quat::from_rotation_y(camera_transform.rotation.to_euler(EulerRot::YXZ).0);

    for event in movement_event_reader.read() {
        for (_, jump_impulse, mut linear_velocity, is_grounded) in &mut controllers {
            match event {
                MovementAction::Move(movement, _) => {
                    if movement.length_squared() > 0.0 {
                        // Convert the input direction to be relative to camera orientation
                        let movement_local = Vec3::new(movement.x, 0.0, -movement.y);

                        // Then rotate it by the camera's yaw
                        let movement_world = camera_yaw * movement_local;

                        // Apply movement with player's current speed
                        linear_velocity.x = movement_world.x * player_speed * delta_time;
                        linear_velocity.z = movement_world.z * player_speed * delta_time;
                    }
                }
                MovementAction::Jump => {
                    if is_grounded {
                        linear_velocity.y = jump_impulse.0;
                    }
                }
            }
        }
    }
}

// Update the apply_movement_damping function for consistency
fn apply_movement_damping(
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