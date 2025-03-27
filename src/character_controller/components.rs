use avian3d::collision::Collider;
use avian3d::math::{Quaternion, Scalar, Vector};
use avian3d::prelude::{LockedAxes, RigidBody, ShapeCaster};
use bevy::math::Dir3;
use bevy::prelude::Component;

/// A marker component indicating that an entity is using a character controller.
/// Requires all components needed for the controller to function properly.
#[derive(Component)]
#[require(
    RigidBody,
    Collider,
    ShapeCaster,
    LockedAxes,
    MovementAcceleration,
    MovementDampingFactor,
    JumpImpulse,
    MaxSlopeAngle
)]
pub struct CharacterController;

/// A marker component indicating that an entity is on the ground.
#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct Grounded;

/// The maximum angle a slope can have for a character controller
/// to be able to climb and jump. If the slope is steeper than this angle,
/// the character will slide down.
#[derive(Component, Default)]
pub struct MaxSlopeAngle(pub(crate) Scalar);

/// The acceleration used for character movement.
#[derive(Component, Default)]
pub struct MovementAcceleration(pub Scalar);

/// The damping factor used for slowing down movement.
#[derive(Component, Default)]
pub struct MovementDampingFactor(pub Scalar);

/// The strength of a jump.
#[derive(Component, Default)]
pub struct JumpImpulse(pub Scalar);

// Helper functions to create a character controller

impl CharacterController {
    pub fn new(collider: Collider) -> (
        Self,
        RigidBody,
        Collider,
        ShapeCaster,
        LockedAxes,
        MovementAcceleration,
        MovementDampingFactor,
        JumpImpulse,
        MaxSlopeAngle,
    ) {
        // Create shape caster as a slightly smaller version of collider
        let mut caster_shape = collider.clone();
        caster_shape.set_scale(Vector::ONE * 0.99, 10);

        (
            CharacterController,
            RigidBody::Dynamic,
            collider,
            ShapeCaster::new(
                caster_shape,
                Vector::ZERO,
                Quaternion::default(),
                Dir3::NEG_Y,
            )
                .with_max_distance(0.2),
            LockedAxes::ROTATION_LOCKED,
            MovementAcceleration(30.0),
            MovementDampingFactor(0.9),
            JumpImpulse(7.0),
            MaxSlopeAngle((30.0 as Scalar).to_radians()),
        )
    }

    pub fn with_movement(
        collider: Collider,
        acceleration: Scalar,
        damping: Scalar,
        jump_impulse: Scalar,
        max_slope_angle: Scalar,
    ) -> (
        Self,
        RigidBody,
        Collider,
        ShapeCaster,
        LockedAxes,
        MovementAcceleration,
        MovementDampingFactor,
        JumpImpulse,
        MaxSlopeAngle,
    ) {
        // Create shape caster as a slightly smaller version of collider
        let mut caster_shape = collider.clone();
        caster_shape.set_scale(Vector::ONE * 0.99, 10);

        (
            CharacterController,
            RigidBody::Dynamic,
            collider,
            ShapeCaster::new(
                caster_shape,
                Vector::ZERO,
                Quaternion::default(),
                Dir3::NEG_Y,
            )
                .with_max_distance(0.2),
            LockedAxes::ROTATION_LOCKED,
            MovementAcceleration(acceleration),
            MovementDampingFactor(damping),
            JumpImpulse(jump_impulse),
            MaxSlopeAngle(max_slope_angle),
        )
    }
}