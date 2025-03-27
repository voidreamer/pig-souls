use bevy::prelude::*;
use avian3d::prelude::*;
use std::time::Duration;
use rand::Rng;
use crate::game_states::AppState;

// Plugin to handle all the breakable prop functionality
pub struct BreakablePropsPlugin;

impl Plugin for BreakablePropsPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Breakable>()
            .register_type::<BrokenPiece>()
            .add_event::<BreakPropEvent>()
            .add_systems(OnEnter(AppState::InGame), setup)
            .add_systems(FixedUpdate, (
                detect_breakable_collisions,
                break_props,
                despawn_broken_pieces,
            ).run_if(in_state(AppState::InGame)));
    }
}

// Component to mark entities as breakable
#[derive(Component, Reflect, Default)]
#[reflect(Component)]
pub struct Breakable {
    // Minimum impulse required to break the prop
    pub break_threshold: f32,
    // Handles to the broken pieces' scene or mesh
    pub broken_pieces: Vec<Handle<Scene>>,
    // Initial impulse to apply to the pieces when broken
    pub explosion_force: f32,
    // How long the pieces should exist before despawning
    pub despawn_delay: f32,
}

// Component to mark and track broken pieces
#[derive(Component, Reflect, Default)]
#[reflect(Component)]
pub struct BrokenPiece {
    pub timer: Timer,
}

// Event to trigger when a prop should break
#[derive(Event)]
pub struct BreakPropEvent {
    pub entity: Entity,
    pub impact_point: Vec3,
    pub impact_force: f32,
}

// System to detect collisions with breakable props
fn detect_breakable_collisions(
    mut collision_events: EventReader<Collision>,
    mut break_events: EventWriter<BreakPropEvent>,
    breakables: Query<&Breakable>,
    rigid_bodies: Query<&RigidBody>,
) {
    for collision in collision_events.read() {
        // Assuming Collision is a wrapper around a Contacts struct
        // Extract the contacts from the collision
        let contacts = &collision.0;

        // Check if either entity is breakable
        let (breakable_entity, other_entity) = if breakables.contains(contacts.entity1) {
            (contacts.entity1, contacts.entity2)
        } else if breakables.contains(contacts.entity2) {
            (contacts.entity2, contacts.entity1)
        } else {
            continue;
        };

        // Get the breakable component
        let breakable = breakables.get(breakable_entity).unwrap();

        // Only break if the other entity is a dynamic rigid body
        if let Ok(rigid_body) = rigid_bodies.get(other_entity) {
            if *rigid_body != RigidBody::Dynamic {
                continue;
            }
        }

        // Estimate impact force - this is simplified
        // In a real game, you would extract this from the physics data more accurately
        let impact_force = 10.0; // Placeholder - you'd calculate this from the contacts

        // Get a reasonable impact point
        // For now, we'll just use the position of the other entity as the impact point
        // In a real scenario, you'd want to get this from the contact data
        let impact_point = Vec3::ZERO; // Placeholder

        // Only break if force exceeds threshold
        if impact_force >= breakable.break_threshold {
            break_events.send(BreakPropEvent {
                entity: breakable_entity,
                impact_point,
                impact_force,
            });
        }
    }
}

// System to handle breaking props with controlled explosion forces
fn break_props(
    mut commands: Commands,
    mut break_events: EventReader<BreakPropEvent>,
    breakables: Query<(&Breakable, &Transform, &GlobalTransform)>,
) {
    let mut rng = rand::thread_rng();

    for event in break_events.read() {
        if let Ok((breakable, transform, global_transform)) = breakables.get(event.entity) {
            // Despawn the original intact prop
            commands.entity(event.entity).despawn_recursive();

            // Get the original position for better impact point calculation
            let original_pos = global_transform.translation();

            // If we have a zero impact point, use the original position
            let impact_point = if event.impact_point == Vec3::ZERO {
                original_pos
            } else {
                event.impact_point
            };

            // Spawn all the broken pieces
            for piece_scene in &breakable.broken_pieces {
                let piece_entity = commands.spawn((
                    SceneRoot(piece_scene.clone()),
                    Transform::from_matrix(global_transform.compute_matrix()),
                    RigidBody::Dynamic,
                    // Add a simple collider - ideally this would match the piece geometry
                    Collider::cuboid(0.2, 0.2, 0.2), // Placeholder size
                    BrokenPiece {
                        timer: Timer::new(Duration::from_secs_f32(breakable.despawn_delay), TimerMode::Once),
                    },
                    // Add a low restitution for less bounce
                    Restitution::new(0.2),
                    // Add friction to slow down pieces
                    Friction::new(0.8),
                    // Add linear damping to slow down pieces over time
                    LinearDamping(0.5),
                    // Add angular damping to reduce spinning
                    AngularDamping(0.3),
                    // Set a maximum speed to prevent infinite flight
                    MaxLinearSpeed(5.0),
                )).id();

                // Calculate direction from impact point to piece center with a small random offset
                let offset = Vec3::new(
                    rng.gen_range(-0.2..0.2),
                    rng.gen_range(-0.1..0.2),
                    rng.gen_range(-0.2..0.2),
                );
                let piece_pos = original_pos + offset;

                // Direction from impact to piece
                let direction = (piece_pos - impact_point).normalize_or_zero();

                // If direction is zero (rare case), use a random direction
                let direction = if direction.length_squared() < 0.001 {
                    Vec3::new(
                        rng.gen_range(-1.0..1.0),
                        rng.gen_range(0.1..1.0), // Bias upward
                        rng.gen_range(-1.0..1.0),
                    ).normalize()
                } else {
                    direction
                };

                // Create the impulse with constrained force
                let mut impulse = ExternalImpulse::default();

                // Cap the explosion force
                let max_force = 2.0;
                let base_force = breakable.explosion_force.min(max_force);

                // Scale force by distance from impact - closer pieces get more force
                let dist = (piece_pos - impact_point).length();
                let force_scale = (1.0 - (dist * 0.5).min(0.8)) * base_force;

                // Add some randomness but keep it small
                let random_force = Vec3::new(
                    rng.gen_range(-0.2..0.2),
                    rng.gen_range(0.0..0.3), // Bias upward
                    rng.gen_range(-0.2..0.2),
                ) * force_scale * 0.3;

                // Apply the linear impulse with a controlled magnitude
                impulse.apply_impulse(direction * force_scale + random_force);

                // Apply a small torque impulse for rotation
                let offset = Vec3::new(
                    rng.gen_range(-0.05..0.05),
                    rng.gen_range(-0.05..0.05),
                    rng.gen_range(-0.05..0.05),
                );

                impulse.apply_impulse_at_point(
                    Vec3::new(
                        rng.gen_range(-0.1..0.1),
                        rng.gen_range(-0.1..0.1),
                        rng.gen_range(-0.1..0.1),
                    ) * force_scale * 0.1, // Much smaller rotational force
                    offset,
                    Vec3::ZERO
                );

                commands.entity(piece_entity).insert(impulse);
            }
        }
    }
}
// System to despawn broken pieces after their timer expires
fn despawn_broken_pieces(
    mut commands: Commands,
    mut pieces: Query<(Entity, &mut BrokenPiece)>,
    time: Res<Time>,
) {
    for (entity, mut piece) in &mut pieces {
        piece.timer.tick(time.delta());

        if piece.timer.finished() {
            commands.entity(entity).despawn_recursive();
        }
    }
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    // Spawn a breakable vase
    commands.spawn((
        SceneRoot(asset_server.load("models/vase.glb#Scene0")),
        Transform::from_xyz(-5.0, 1.0, 0.0),
        Collider::capsule(0.5, 0.3), // Approximate vase shape
        RigidBody::Dynamic,
        // Add a sleeping component to keep it stable until hit
        Breakable {
            break_threshold: 2.0,  // Lower threshold so it breaks more easily
            broken_pieces: vec![
                asset_server.load("models/vase_piece1.glb#Scene0"),
                asset_server.load("models/vase_piece2.glb#Scene0"),
                asset_server.load("models/vase_piece3.glb#Scene0"),
                asset_server.load("models/vase_piece4.glb#Scene0"),
            ],
            explosion_force: 1.0,  // Lower explosion force (was 3.0)
            despawn_delay: 5.0,
        },
    ));

    // Spawn a "weapon" or object to hit the vase with
    commands.spawn((
        Transform::from_xyz(3.0, 1.0, 0.0),
        Collider::sphere(0.3),
        RigidBody::Dynamic,
        Mass(5.0), // Make it heavy
        // Start with no impulse - can add it later or control it manually
    ));
}
