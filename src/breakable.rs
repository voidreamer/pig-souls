use bevy::prelude::*;
use avian3d::prelude::*;
use std::time::Duration;
use bevy::gltf::{GltfMesh, GltfNode};
use rand::prelude::IteratorRandom;
use rand::Rng;
use crate::game_states::AppState;

/// Plugin to handle all breakable prop functionality in the game
pub struct BreakablePropsPlugin;

impl Plugin for BreakablePropsPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Breakable>()
            .register_type::<BrokenPiece>()
            .register_type::<ImpactSettings>()
            .register_type::<ProceduralBreakSettings>()
            .register_type::<GltfBreakPattern>()
            .register_type::<FracturePattern>()
            .add_event::<BreakPropEvent>()
            .add_systems(OnEnter(AppState::InGame), setup)
            .add_systems(FixedUpdate, (
                detect_breakable_collisions,
                break_props.after(detect_breakable_collisions),
                despawn_broken_pieces,
            ).run_if(in_state(AppState::InGame)));
    }
}

/// Primary component to mark entities as breakable
#[derive(Component, Reflect, Default)]
#[reflect(Component)]
#[require(RigidBody)] // All breakable objects must be rigid bodies
struct Breakable {
    /// Minimum impulse required to break the prop
    pub break_threshold: f32,
    /// Handles to the broken pieces' scene or mesh
    pub broken_pieces: Vec<Handle<Scene>>,
    /// Initial impulse to apply to the pieces when broken
    pub explosion_force: f32,
    /// How long the pieces should exist before despawning
    pub despawn_delay: f32,
}

/// Component to control procedural breaking settings
#[derive(Component, Reflect, Default)]
#[reflect(Component)]
pub struct ProceduralBreakSettings {
    pub piece_count: u32,
    pub color: Color,
    pub size_multiplier: f32,
    pub shape_distribution: ShapeDistribution,
    pub max_size_variation: f32,
    pub inner_color: Option<Color>, // For pieces from the "inside" of the object
    pub maintain_proportion: bool,  // Keep pieces proportional to original object
}

#[derive(Reflect, Default)]
pub enum ShapeDistribution {
    #[default]
    Random,
    Mostly(ShapeType),
    Only(ShapeType),
    Custom(Vec<(ShapeType, f32)>), // Shape type with weight
}

#[derive(Reflect, Default)]
pub enum ShapeType {
    #[default]
    Cube,
    Sphere,
    Cylinder,
    Cone,
    Tetrahedron,
    Custom(Handle<Mesh>),
}

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct FracturePattern {
    pub pattern_type: PatternType,
    pub center_bias: f32,       // How much pieces cluster toward center
    pub impact_alignment: f32,  // How much break aligns with impact direction
    pub size_distribution: SizeDistribution,
}

#[derive(Reflect)]
pub enum PatternType {
    Radial,         // Pieces radiate from center
    Layered,        // Pieces in layers (like an onion)
    Linear,         // Pieces along a line
    Voronoi,        // Natural looking random breaks
    Custom(Vec<Transform>), // Custom offsets for each piece
}

#[derive(Reflect)]
pub enum SizeDistribution {
    Uniform,         // All pieces similar size
    GradualIncrease, // Pieces get larger from center
    GradualDecrease, // Pieces get smaller from center
    Random,
}

/// Component for using GLTF models as break pieces
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct GltfBreakPattern {
    pub source: GltfSource,
    pub transform_strategy: TransformStrategy,
    pub piece_count_limit: Option<u32>,
    pub random_selection: bool,
}

#[derive(Reflect)]
pub enum GltfSource {
    // Use nodes from a GLTF file based on naming pattern
    NamedNodes {
        handle: Handle<Gltf>,
        name_pattern: NodePattern,
    },
    // Use pre-defined meshes
    Meshes {
        handles: Vec<Handle<GltfMesh>>,
    },
}

#[derive(Reflect)]
pub enum NodePattern {
    Prefixed {
        prefix: String,      // e.g., "piece_"
        object_name: Option<String>, // Optional object name to filter further
    },
    Named(Vec<String>),      // Specific node names to look for
    All,                     // Use all nodes in the file
}

#[derive(Reflect)]
pub enum TransformStrategy {
    PreserveOriginal,        // Use transforms as defined in GLTF
    RandomizeRotation,       // Keep positions but randomize rotations
    CenterAndExplode,        // Center all pieces and apply explosion force
    AlignWithImpact,         // Align breaking direction with impact
}

/// Component to control impact and physics settings
#[derive(Component, Reflect, Clone)]
#[reflect(Component)]
#[require(Sleeping)] // Objects with impact settings start in sleeping state
struct ImpactSettings {
    /// Maximum distance pieces can travel before despawning
    pub max_scatter_distance: f32,
    /// Whether to play impact sound when broken
    pub play_sound: bool,
    /// Whether to spawn particles when broken
    pub spawn_particles: bool,
    /// Restitution value for broken pieces
    pub piece_restitution: f32,
    /// Friction value for broken pieces
    pub piece_friction: f32,
    /// Linear damping for pieces
    pub piece_linear_damping: f32,
    /// Angular damping for pieces
    pub piece_angular_damping: f32,
}

impl Default for ImpactSettings {
    fn default() -> Self {
        Self {
            max_scatter_distance: 5.0,
            play_sound: true,
            spawn_particles: true,
            piece_restitution: 0.2,
            piece_friction: 0.8,
            piece_linear_damping: 0.5,
            piece_angular_damping: 0.3,
        }
    }
}

/// Component to mark and track broken pieces
#[derive(Component, Reflect)]
#[reflect(Component)]
#[require(LinearDamping, AngularDamping, Restitution, Friction, RigidBody)] // Pieces always have physics components
struct BrokenPiece {
    pub timer: Timer,
    pub original_position: Vec3,
    pub max_distance: f32,
}

impl Default for BrokenPiece {
    fn default() -> Self {
        Self {
            timer: Timer::new(Duration::from_secs_f32(5.0), TimerMode::Once),
            original_position: Vec3::ZERO,
            max_distance: 5.0,
        }
    }
}

/// Event to trigger when a prop should break
#[derive(Event)]
pub struct BreakPropEvent {
    pub entity: Entity,
    pub impact_point: Vec3,
    pub impact_force: f32,
    pub impact_velocity: Vec3,
}

/// System to detect collisions with breakable props
fn detect_breakable_collisions(
    mut collision_events: EventReader<Collision>,
    mut break_events: EventWriter<BreakPropEvent>,
    breakables: Query<&Breakable>,
    transforms: Query<&GlobalTransform>,
    rigid_bodies: Query<&RigidBody>,
    velocities: Query<&LinearVelocity>,
) {
    for collision in collision_events.read() {
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

        // Calculate impact force based on velocity of the other entity
        let impact_force = if let Ok(vel) = velocities.get(other_entity) {
            vel.0.length() * 2.0 // Scale factor to convert velocity to approximate force
        } else {
            3.0 // Default force if velocity isn't available
        };

        // Only break if force exceeds threshold
        if impact_force < breakable.break_threshold {
            continue;
        }

        // Get impact velocity for effect scaling
        let impact_velocity = velocities.get(other_entity)
            .map(|vel| vel.0)
            .unwrap_or(Vec3::ZERO);

        // Get impact point from transforms
        let impact_point = if let (Ok(transform1), Ok(transform2)) = (
            transforms.get(contacts.entity1),
            transforms.get(contacts.entity2)
        ) {
            // Use midpoint between entities as approximate impact point
            (transform1.translation() + transform2.translation()) * 0.5
        } else if let Ok(transform) = transforms.get(breakable_entity) {
            // Fallback to breakable object's position
            transform.translation()
        } else {
            Vec3::ZERO
        };

        // Send break event
        break_events.send(BreakPropEvent {
            entity: breakable_entity,
            impact_point,
            impact_force,
            impact_velocity,
        });
    }
}

/// System to handle breaking props with improved physics and effects
fn break_props(
    mut commands: Commands,
    mut break_events: EventReader<BreakPropEvent>,
    breakables: Query<(
        Entity,
        &Breakable,
        &GlobalTransform,
        Option<&ImpactSettings>,
        Option<&ProceduralBreakSettings>,
        Option<&GltfBreakPattern>
    )>,
    asset_server: Res<AssetServer>,
    gltf_assets: Res<Assets<Gltf>>,
    gltf_meshes: Res<Assets<GltfMesh>>,
    gltf_nodes: Res<Assets<GltfNode>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut rng = rand::thread_rng();

    for event in break_events.read() {
        if let Ok((
                  entity,
                  breakable,
                  global_transform,
                  impact_settings,
                  procedural_settings,
                  gltf_pattern
              )) =
            breakables.get(event.entity)
        {
            // Get default settings or use custom ones
            let impact = impact_settings.cloned().unwrap_or_default();

            // Despawn the original intact prop
            commands.entity(entity).despawn_recursive();

            // Get the original position
            let original_pos = global_transform.translation();

            // If we have a valid impact point, use it, otherwise use the original position
            let impact_point = if event.impact_point == Vec3::ZERO {
                original_pos
            } else {
                event.impact_point
            };

            // Priority 1: Check if we have a GLTF break pattern
            if let Some(gltf_pattern) = gltf_pattern {
                // Use GLTF-based breaking
                spawn_gltf_pieces(
                    &mut commands,
                    gltf_pattern,
                    &gltf_assets,
                    &gltf_meshes,
                    &gltf_nodes,
                    global_transform,
                    breakable,
                    impact_point,
                    event.impact_force,
                    &impact,
                    &mut rng,
                );
            }
            // Priority 2: Check if we have model pieces to spawn
            else if !breakable.broken_pieces.is_empty() {
                spawn_model_pieces(
                    &mut commands,
                    &breakable.broken_pieces,
                    breakable,
                    global_transform,
                    impact_point,
                    event.impact_force,
                    &impact,
                    &mut rng,
                );
            }
            // Priority 3: If we need procedural pieces
            else if let Some(proc_settings) = procedural_settings {
                if proc_settings.piece_count > 0 {
                    let piece_material = materials.add(StandardMaterial {
                        base_color: proc_settings.color,
                        perceptual_roughness: 0.8,
                        ..default()
                    });

                    spawn_procedural_pieces(
                        &mut commands,
                        &mut meshes,
                        piece_material,
                        proc_settings.piece_count,
                        proc_settings.size_multiplier,
                        breakable,
                        global_transform,
                        impact_point,
                        event.impact_force,
                        &impact,
                        &mut rng,
                    );
                }
            }

            // Optional: Spawn particles at impact point
            if impact.spawn_particles {
                spawn_break_particles(
                    &mut commands,
                    &mut meshes,
                    impact_point,
                    event.impact_velocity,
                );
            }

            // Play break sound
            if impact.play_sound {
                commands.spawn(AudioPlayer::new(asset_server.load("sounds/breaking.ogg")));
            }
        }
    }
}

/// Helper function to spawn model-based broken pieces
fn spawn_model_pieces(
    commands: &mut Commands,
    pieces: &[Handle<Scene>],
    breakable: &Breakable,
    global_transform: &GlobalTransform,
    impact_point: Vec3,
    impact_force: f32,
    impact: &ImpactSettings,
    rng: &mut impl Rng,
) {
    let original_pos = global_transform.translation();

    for piece_scene in pieces {
        // Small random offset to prevent pieces from spawning at the exact same spot
        let offset = Vec3::new(
            rng.gen_range(-0.1..0.1),
            rng.gen_range(-0.05..0.1),
            rng.gen_range(-0.1..0.1),
        );

        let piece_pos = original_pos + offset;

        // Spawn the piece - we use required components for the physics properties!
        let piece_entity = commands.spawn((
            SceneRoot(piece_scene.clone()),
            Transform::from_matrix(global_transform.compute_matrix())
                .with_translation(piece_pos),
            // Required components will handle physics setup
            BrokenPiece {
                timer: Timer::new(Duration::from_secs_f32(breakable.despawn_delay), TimerMode::Once),
                original_position: original_pos,
                max_distance: impact.max_scatter_distance,
            },
            // These will override the defaults from BrokenPiece's required components
            LinearDamping(impact.piece_linear_damping),
            AngularDamping(impact.piece_angular_damping),
            Restitution::new(impact.piece_restitution),
            Friction::new(impact.piece_friction),
            // Add a simple collider
            Collider::cuboid(0.2, 0.2, 0.2),
            MaxLinearSpeed(5.0),
        )).id();

        apply_explosion_impulse(
            commands,
            piece_entity,
            piece_pos,
            impact_point,
            breakable.explosion_force,
            impact_force,
            rng,
        );
    }
}

/// Helper function to spawn procedurally generated broken pieces
fn spawn_procedural_pieces(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    material: Handle<StandardMaterial>,
    count: u32,
    size_multiplier: f32,
    breakable: &Breakable,
    global_transform: &GlobalTransform,
    impact_point: Vec3,
    impact_force: f32,
    impact: &ImpactSettings,
    rng: &mut impl Rng,
) {
    let original_pos = global_transform.translation();
    let scale = global_transform.scale();
    let avg_scale = (scale.x + scale.y + scale.z) / 3.0 * size_multiplier;

    for _ in 0..count {
        // Random offset based on original object scale
        let offset = Vec3::new(
            rng.gen_range(-0.2..0.2) * avg_scale,
            rng.gen_range(-0.1..0.3) * avg_scale,
            rng.gen_range(-0.2..0.2) * avg_scale,
        );

        let piece_pos = original_pos + offset;

        // Random size for piece
        let size = Vec3::new(
            rng.gen_range(0.05..0.15) * avg_scale,
            rng.gen_range(0.05..0.15) * avg_scale,
            rng.gen_range(0.05..0.15) * avg_scale,
        );

        // Create mesh based on random shape type
        let mesh = match rng.gen_range(0..3) {
            0 => meshes.add(Cuboid::new(size.x, size.y, size.z)),
            1 => meshes.add(Sphere::new(size.x.min(size.y).min(size.z))),
            _ => meshes.add(Cylinder::new(size.y, size.x.min(size.z))),
        };

        // Random rotation for variety
        let rotation = Quat::from_euler(
            EulerRot::XYZ,
            rng.gen_range(-std::f32::consts::PI..std::f32::consts::PI),
            rng.gen_range(-std::f32::consts::PI..std::f32::consts::PI),
            rng.gen_range(-std::f32::consts::PI..std::f32::consts::PI),
        );

        // Spawn the piece using required components
        let piece_entity = commands.spawn((
            Transform::from_translation(piece_pos).with_rotation(rotation),
            Mesh3d(mesh),
            MeshMaterial3d(material.clone()),
            // BrokenPiece requires RigidBody, LinearDamping, AngularDamping, etc.
            BrokenPiece {
                timer: Timer::new(Duration::from_secs_f32(breakable.despawn_delay), TimerMode::Once),
                original_position: original_pos,
                max_distance: impact.max_scatter_distance,
            },
            // These will override the defaults from BrokenPiece
            LinearDamping(impact.piece_linear_damping),
            AngularDamping(impact.piece_angular_damping),
            Restitution::new(impact.piece_restitution),
            Friction::new(impact.piece_friction),
            Collider::cuboid(size.x, size.y, size.z),
            MaxLinearSpeed(5.0),
        )).id();

        apply_explosion_impulse(
            commands,
            piece_entity,
            piece_pos,
            impact_point,
            breakable.explosion_force * 0.6, // Less force for procedural pieces
            impact_force,
            rng,
        );
    }
}

/// Helper function to apply controlled explosion impulse to pieces
fn apply_explosion_impulse(
    commands: &mut Commands,
    entity: Entity,
    piece_pos: Vec3,
    impact_point: Vec3,
    explosion_force: f32,
    impact_force: f32,
    rng: &mut impl Rng,
) {
    // Direction from impact to piece
    let direction = (piece_pos - impact_point).normalize_or_zero();

    // If direction is zero, use a random direction
    let direction = if direction.length_squared() < 0.001 {
        Vec3::new(
            rng.gen_range(-1.0..1.0),
            rng.gen_range(0.1..1.0), // Bias upward
            rng.gen_range(-1.0..1.0),
        ).normalize()
    } else {
        direction
    };

    let mut impulse = ExternalImpulse::default();

    // Calculate distance-based scaling
    let dist = (piece_pos - impact_point).length();

    // Hard cap on maximum force
    let max_force = 2.0;
    // Scale by impact force but keep within max bounds
    let adjusted_force = ((explosion_force * (impact_force / 10.0).min(1.5))).min(max_force);
    // Apply distance falloff
    let base_force = adjusted_force * (1.0 - (dist * 0.5).min(0.8));

    // Randomized but controlled force
    let random_force = Vec3::new(
        rng.gen_range(-0.15..0.15),
        rng.gen_range(0.0..0.2),  // Upward bias
        rng.gen_range(-0.15..0.15),
    ) * base_force * 0.3;

    // Apply the main impulse
    impulse.apply_impulse(direction * base_force + random_force);

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
        ) * base_force * 0.1,
        offset,
        Vec3::ZERO
    );

    commands.entity(entity).insert(impulse);
}

/// Helper function to spawn particles at the break point
fn spawn_break_particles(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    position: Vec3,
    velocity: Vec3,
) {
    // This is a simplified version - you'd typically use a particle system
    let particle_count = 8;
    let particle_size = 0.05;

    for _ in 0..particle_count {
        let velocity_direction = velocity.normalize_or_zero();
        let mut rng = rand::thread_rng();

        // Random direction biased toward the impact velocity
        let random_dir = Vec3::new(
            rng.gen_range(-1.0..1.0),
            rng.gen_range(0.0..1.0),
            rng.gen_range(-1.0..1.0),
        ).normalize();

        let direction = if velocity_direction.length_squared() > 0.001 {
            (velocity_direction + random_dir * 0.5).normalize()
        } else {
            random_dir
        };

        // Spawn a small particle with physics
        commands.spawn((
            Transform::from_translation(position),
            Mesh3d(meshes.add(Sphere::new(particle_size * rng.gen_range(0.5..1.0)))),
            // BrokenPiece requires all the physics components
            BrokenPiece {
                timer: Timer::new(Duration::from_secs_f32(1.5), TimerMode::Once),
                original_position: position,
                max_distance: 10.0,
            },
            // Override with particle-specific settings
            LinearDamping(0.8),
            Collider::sphere(particle_size * 0.5),
            ExternalImpulse::new(direction * rng.gen_range(0.5..1.5)),
        ));
    }
}

/// System to despawn broken pieces after their timer expires or if they travel too far
fn despawn_broken_pieces(
    mut commands: Commands,
    mut pieces: Query<(Entity, &mut BrokenPiece, &GlobalTransform)>,
    time: Res<Time>,
) {
    for (entity, mut piece, transform) in &mut pieces {
        piece.timer.tick(time.delta());

        // Calculate distance from original position
        let distance_from_origin = (transform.translation() - piece.original_position).length();

        // Despawn if timer finished or if piece traveled too far
        if piece.timer.finished() || distance_from_origin > piece.max_distance {
            if commands.get_entity(entity).is_some() {
                commands.entity(entity).despawn_recursive();
            }
        }
    }
}

/// Example usage in game setup
fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Creating a breakable vase with GLTF node-based pieces
    commands.spawn((
        SceneRoot(asset_server.load("models/intact_vase.glb#Scene0")),
        Transform::from_xyz(-5.0, 1.0, 0.0),
        Collider::capsule(0.5, 0.3),
        Breakable {
            break_threshold: 2.0,
            broken_pieces: vec![],
            explosion_force: 1.0,
            despawn_delay: 8.0,
        },
        GltfBreakPattern {
            source: GltfSource::NamedNodes {
                handle: asset_server.load("models/broken_vase.glb"),
                name_pattern: NodePattern::Prefixed {
                    prefix: "piece_".to_string(),
                    object_name: Some("vase".into()),
                },
            },
            transform_strategy: TransformStrategy::AlignWithImpact,
            piece_count_limit: Some(10),
            random_selection: true,
        },
        ImpactSettings::default(),
    ));

    // Add procedural breakable objects
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(0.4))),
        MeshMaterial3d(materials.add(Color::srgb(0.8, 0.4, 0.3))),
        Transform::from_xyz(-2.0, 1.0, -1.0),
        Collider::sphere(0.4),
        Breakable {
            break_threshold: 1.5,
            broken_pieces: vec![],
            explosion_force: 0.8,
            despawn_delay: 4.0,
        },
        ProceduralBreakSettings {
            piece_count: 8,
            color: Color::srgb(0.8, 0.4, 0.3),
            size_multiplier: 1.0,
            shape_distribution: ShapeDistribution::Random,
            max_size_variation: 0.5,
            inner_color: None,
            maintain_proportion: true,
        },
        ImpactSettings::default(),
    ));

    // Add a crate with different breaking properties
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.5, 0.5, 0.5))),
        MeshMaterial3d(materials.add(Color::srgb(0.6, 0.4, 0.2))),
        Transform::from_xyz(8.0, 1.0, -2.0),
        Collider::cuboid(0.25, 0.25, 0.25),
        Breakable {
            break_threshold: 2.5,
            broken_pieces: vec![],
            explosion_force: 1.2,
            despawn_delay: 5.0,
        },
        ProceduralBreakSettings {
            piece_count: 12,
            color: Color::srgb(0.6, 0.4, 0.2),
            size_multiplier: 0.8,
            shape_distribution: ShapeDistribution::Only(ShapeType::Cube),
            max_size_variation: 0.3,
            inner_color: Some(Color::srgb(0.5, 0.3, 0.1)),
            maintain_proportion: true,
        },
        ImpactSettings {
            max_scatter_distance: 6.0,
            piece_restitution: 0.1,
            piece_friction: 0.9,
            piece_linear_damping: 0.6,
            piece_angular_damping: 0.4,
            ..default()
        },
    ));
}
/// Helper function to spawn pieces from GLTF nodes
fn spawn_gltf_pieces(
    commands: &mut Commands,
    gltf_break_pattern: &GltfBreakPattern,
    gltf_assets: &Res<Assets<Gltf>>,
    gltf_meshes: &Res<Assets<GltfMesh>>,
    gltf_nodes: &Res<Assets<GltfNode>>,
    original_transform: &GlobalTransform,
    breakable: &Breakable,
    impact_point: Vec3,
    impact_force: f32,
    impact: &ImpactSettings,
    rng: &mut impl Rng,
) {
    let original_pos = original_transform.translation();

    match &gltf_break_pattern.source {
        GltfSource::NamedNodes { handle, name_pattern } => {
            if let Some(gltf) = gltf_assets.get(handle) {
                // Get node handles based on pattern
                let mut node_handles = Vec::new();

                match name_pattern {
                    NodePattern::Prefixed { prefix, object_name } => {
                        // Filter nodes by prefix and optional object name
                        for (name, node_handle) in &gltf.named_nodes {
                            let matches_prefix = name.starts_with(prefix);
                            let matches_object = object_name
                                .as_ref()
                                .map_or(true, |obj_name| name.contains(obj_name));

                            if matches_prefix && matches_object {
                                node_handles.push(node_handle.clone());
                            }
                        }
                    },
                    NodePattern::Named(names) => {
                        // Get specifically named nodes
                        for name in names {
                            if let Some(node_handle) = gltf.named_nodes.get(name.as_str()) {
                                node_handles.push(node_handle.clone());
                            }
                        }
                    },
                    NodePattern::All => {
                        // Get all nodes (though this might include non-piece nodes)
                        node_handles = gltf.nodes.clone();
                    }
                }

                // Apply piece count limit if specified
                if let Some(limit) = gltf_break_pattern.piece_count_limit {
                    if gltf_break_pattern.random_selection && node_handles.len() > limit as usize {
                        // Randomly select nodes
                        node_handles = node_handles
                            .into_iter()
                            .choose_multiple(rng, limit as usize);
                    } else {
                        // Take first N nodes
                        node_handles.truncate(limit as usize);
                    }
                }

                // Spawn each piece
                for node_handle in node_handles {
                    if let Some(node) = gltf_nodes.get(&node_handle) {
                        // Only process nodes that have a mesh
                        if let Some(mesh_handle) = node.mesh.as_ref() {
                            if let Some(mesh) = gltf_meshes.get(mesh_handle) {
                                // Calculate position based on transform strategy
                                let (position, rotation) = calculate_piece_transform(
                                    original_transform,
                                    &node.transform,
                                    impact_point,
                                    &gltf_break_pattern.transform_strategy,
                                    rng,
                                );

                                // Spawn the piece with the node's mesh
                                let piece_entity = commands.spawn((
                                    // Use Mesh3d and MeshMaterial3d for rendering
                                    Mesh3d(mesh.primitives[0].mesh.clone()),
                                    MeshMaterial3d(mesh.primitives[0].material.clone().unwrap_or_default()),
                                    Transform::from_translation(position).with_rotation(rotation),
                                    BrokenPiece {
                                        timer: Timer::new(Duration::from_secs_f32(breakable.despawn_delay), TimerMode::Once),
                                        original_position: original_pos,
                                        max_distance: impact.max_scatter_distance,
                                    },
                                    LinearDamping(impact.piece_linear_damping),
                                    AngularDamping(impact.piece_angular_damping),
                                    Restitution::new(impact.piece_restitution),
                                    Friction::new(impact.piece_friction),
                                    // Use a simple collider
                                    Collider::cuboid(0.15, 0.15, 0.15),
                                    MaxLinearSpeed(5.0),
                                )).id();

                                // Apply explosion impulses
                                apply_explosion_impulse(
                                    commands,
                                    piece_entity,
                                    position,
                                    impact_point,
                                    breakable.explosion_force,
                                    impact_force,
                                    rng,
                                );
                            }
                        }
                    }
                }
            }
        },
        GltfSource::Meshes { handles: _ } => {
            // Implementation for direct mesh handles could be added here if needed
        }
    }
}

/// Helper function to calculate piece transforms based on strategy
fn calculate_piece_transform(
    original_transform: &GlobalTransform,
    node_transform: &Transform,
    impact_point: Vec3,
    strategy: &TransformStrategy,
    rng: &mut impl Rng,
) -> (Vec3, Quat) {
    let original_pos = original_transform.translation();

    match strategy {
        TransformStrategy::PreserveOriginal => {
            // Apply original transform with node's transform
            let node_global = original_transform.mul_transform(*node_transform);
            (node_global.translation(), node_global.rotation())
        },
        TransformStrategy::RandomizeRotation => {
            // Keep position from node but randomize rotation
            let node_global = original_transform.mul_transform(*node_transform);
            let rotation = Quat::from_euler(
                EulerRot::XYZ,
                rng.gen_range(-std::f32::consts::PI..std::f32::consts::PI),
                rng.gen_range(-std::f32::consts::PI..std::f32::consts::PI),
                rng.gen_range(-std::f32::consts::PI..std::f32::consts::PI),
            );
            (node_global.translation(), rotation)
        },
        TransformStrategy::CenterAndExplode => {
            // Position pieces with more dramatic offsets
            let piece_local_pos = node_transform.translation;
            let direction = piece_local_pos.normalize_or_zero();
            let direction = if direction.length_squared() < 0.001 {
                Vec3::new(
                    rng.gen_range(-1.0..1.0),
                    rng.gen_range(0.1..1.0),
                    rng.gen_range(-1.0..1.0),
                ).normalize()
            } else {
                direction
            };

            let offset = direction * (piece_local_pos.length() * 0.5 + rng.gen_range(0.1..0.3));
            let rotation = Quat::from_euler(
                EulerRot::XYZ,
                rng.gen_range(-std::f32::consts::PI..std::f32::consts::PI),
                rng.gen_range(-std::f32::consts::PI..std::f32::consts::PI),
                rng.gen_range(-std::f32::consts::PI..std::f32::consts::PI),
            );
            (original_pos + offset, rotation)
        },
        TransformStrategy::AlignWithImpact => {
            // Calculate direction from impact to original position
            let impact_dir = (original_pos - impact_point).normalize_or_zero();
            let piece_local_pos = node_transform.translation;

            // Use impact direction but preserve relative position of piece
            let local_dir = piece_local_pos.normalize_or_zero();
            let direction = if impact_dir.length_squared() > 0.001 {
                if local_dir.length_squared() > 0.001 {
                    // Blend impact direction with local direction
                    (impact_dir + local_dir * 0.5).normalize()
                } else {
                    impact_dir
                }
            } else if local_dir.length_squared() > 0.001 {
                local_dir
            } else {
                Vec3::new(
                    rng.gen_range(-1.0..1.0),
                    rng.gen_range(0.1..1.0),
                    rng.gen_range(-1.0..1.0),
                ).normalize()
            };

            // Create offset and rotation aligned with impact
            let distance = piece_local_pos.length();
            let offset = direction * (distance + rng.gen_range(0.05..0.2));
            let base_rotation = original_transform.rotation();
            let additional_rotation = Quat::from_rotation_arc(Vec3::Y, direction);
            (original_pos + offset, base_rotation * additional_rotation)
        }
    }
}